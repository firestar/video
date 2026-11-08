//! Job processing: FFmpeg transcode pipeline.
//!
//! Pipeline: download source from R2 → ffprobe duration → ffmpeg transcode
//! → ffmpeg thumbnail → upload both to R2 → finalize DB row.
//!
//! Retries up to `config.max_retries` attempts per job; on final failure
//! marks the video `failed`.

use std::time::Duration;

use anyhow::bail;
use db::videos;
use domain::VideoStatus;
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;
use tracing::{info, warn};

use crate::queue::{JobQueue, TranscodeJob};
use crate::WorkerState;

/// Parse `ffprobe -v quiet -print_format json -show_format -show_streams -i <file>`
/// output into `(duration_secs, video_width, video_height)`.
fn parse_probe(probe_stdout: &str) -> anyhow::Result<(f64, i32, i32)> {
    let v: Value = serde_json::from_str(probe_stdout)
        .map_err(|e| anyhow::anyhow!("ffprobe JSON parse: {e}"))?;
    let dur = v
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .ok_or_else(|| anyhow::anyhow!("ffprobe output missing format.duration"))?;
    let dur = dur
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("ffprobe duration parse: {e}"))?;
    let (w, h) = v
        .get("streams")
        .and_then(|s| s.as_array())
        .and_then(|s| {
            s.iter()
                .find(|st| st.get("codec_type").and_then(|c| c.as_str()) == Some("video"))
        })
        .map(|st| {
            (
                st.get("width").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                st.get("height").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
            )
        })
        .filter(|(w, h)| *w > 0 && *h > 0)
        .ok_or_else(|| anyhow::anyhow!("ffprobe output missing video stream dimensions"))?;
    Ok((dur, w, h))
}

/// Run ffprobe on `input`, return `(duration_secs, width, height)`.
async fn probe(ffprobe: &str, input: &std::path::Path) -> anyhow::Result<(f64, i32, i32)> {
    let out = Command::new(ffprobe)
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg("-i")
        .arg(input)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("ffprobe spawn: {e}"))?;
    if !out.status.success() {
        bail!(
            "ffprobe failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    parse_probe(&String::from_utf8_lossy(&out.stdout))
}

/// Quality ladder (heights in px), highest first. The top rung is always the
/// source's native height (capped at 1080p — never upscale); the rest are the
/// standard rungs strictly below it.
const QUALITY_RUNGS: [i32; 4] = [1080, 720, 480, 360];

/// Choose the quality ladder for a source of `src_height` px, highest first.
/// e.g. 1080 -> [1080, 720, 480, 360]; 540 -> [540, 480, 360]; 240 -> [240].
fn quality_ladder(src_height: i32) -> Vec<i32> {
    let top = src_height.min(1080).max(1);
    let mut ladder: Vec<i32> = QUALITY_RUNGS.iter().copied().filter(|h| *h < top).collect();
    ladder.insert(0, top);
    ladder
}

/// Run ffmpeg transcode: H.264 + AAC, scaled to `height` (native when equal to
/// `src_height`), faststart.
async fn transcode(
    ffmpeg: &str,
    input: &std::path::Path,
    output: &std::path::Path,
    height: i32,
    src_height: i32,
) -> anyhow::Result<()> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("medium")
        .arg("-crf")
        .arg("23");
    if height < src_height {
        cmd.arg("-vf").arg(format!("scale=-2:{height}"));
    }
    let audio_bitrate = if height >= 720 { "192k" } else { "128k" };
    cmd.arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg(audio_bitrate)
        .arg("-movflags")
        .arg("+faststart")
        .arg(output);
    let out = cmd
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("ffmpeg spawn: {e}"))?;
    if !out.status.success() {
        bail!(
            "ffmpeg transcode {height}p failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}


/// Run ffmpeg to extract a JPEG thumbnail at 25% of the duration.
async fn thumbnail(ffmpeg: &str, input: &std::path::Path, duration: f64, out: &std::path::Path) -> anyhow::Result<()> {
    let seek = (duration * 0.25).max(0.0);
    let out = Command::new(ffmpeg)
        .arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-ss")
        .arg(seek.to_string())
        .arg("-vframes")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg(out)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("ffmpeg spawn: {e}"))?;
    if !out.status.success() {
        bail!(
            "ffmpeg thumbnail failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// One full pipeline attempt. Returns `(duration_secs, total_output_bytes)`.
async fn process_once(state: &WorkerState, job: &TranscodeJob) -> anyhow::Result<(i32, i64)> {
    let video_id = job.video_id;
    info!(video_id = %video_id, object_key = %job.object_key, "processing (attempt)");

    // 1. Download source from R2 to a temp file.
    let tmp = TempDir::new().map_err(|e| anyhow::anyhow!("tempdir: {e}"))?;
    let src_path = tmp.path().join("source");
    let thumb_path = tmp.path().join("thumb.jpg");

    let source_bytes = state
        .storage
        .download(&job.object_key)
        .await
        .map_err(|e| anyhow::anyhow!("download source: {e}"))?;
    fs::write(&src_path, &source_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("write source: {e}"))?;
    info!(video_id = %video_id, bytes = source_bytes.len(), "source downloaded");

    // 2. ffprobe → duration + source dimensions.
    let (duration, _src_w, src_h) = probe(&state.config.ffprobe_path, &src_path).await?;
    let ladder = quality_ladder(src_h);
    info!(video_id = %video_id, duration_secs = duration, src_height = src_h, ladder = ?ladder, "probed");

    // 3. Transcode + upload each quality (highest first; rung 0 = native, no scale).
    let mut streams: Vec<Value> = Vec::with_capacity(ladder.len());
    let mut total_size: i64 = 0;
    for h in ladder.iter() {
        let out_path = tmp.path().join(format!("{video_id}_{h}p.mp4"));
        transcode(&state.config.ffmpeg_path, &src_path, &out_path, *h, src_h).await?;

        // Exact output dimensions (scale=-2 rounds the width to even).
        let (_d, w, hh) = probe(&state.config.ffprobe_path, &out_path).await?;

        let out_key = format!("videos/{video_id}_{h}p.mp4");
        let out_bytes = fs::read(&out_path).await.map_err(|e| anyhow::anyhow!("read output: {e}"))?;
        state
            .storage
            .upload(&out_key, out_bytes.clone().into(), "video/mp4")
            .await
            .map_err(|e| anyhow::anyhow!("upload {h}p: {e}"))?;
        total_size += out_bytes.len() as i64;

        streams.push(serde_json::json!({
            "quality": format!("{h}p"),
            "width": w,
            "height": hh,
            "objectKey": out_key,
            "sizeBytes": out_bytes.len() as i64,
        }));
        info!(video_id = %video_id, quality = %format!("{h}p"), bytes = out_bytes.len(), "variant uploaded");
    }

    // 4. Thumbnail at 25% of duration.
    thumbnail(&state.config.ffmpeg_path, &src_path, duration, &thumb_path).await?;

    // 5. Upload thumbnail to R2.
    let thumb_key = format!("thumbnails/{video_id}.jpg");
    let thumb_bytes = fs::read(&thumb_path).await.map_err(|e| anyhow::anyhow!("read thumbnail: {e}"))?;
    state
        .storage
        .upload(&thumb_key, thumb_bytes.into(), "image/jpeg")
        .await
        .map_err(|e| anyhow::anyhow!("upload thumbnail: {e}"))?;

    // 6. Finalize DB row (top quality becomes the canonical object_key).
    let streams_json = Value::Array(streams);
    let top_key = streams_json
        .get(0)
        .and_then(|s| s.get("objectKey"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let row = videos::finalize(
        &state.db,
        video_id,
        VideoStatus::Ready,
        Some(duration as i32),
        Some(total_size),
        state.storage.public_url(&thumb_key).as_deref(),
        Some(&streams_json),
        Some(&top_key),
    )
    .await
    .map_err(|e| anyhow::anyhow!("db finalize: {e}"))?;

    // Fan out a new-upload notification to the channel's subscribers.
    // Best-effort: a notification failure never fails a finished transcode.
    if let Some(row) = row {
        if let Err(e) = db::notifications::notify_new_uploads(&state.db, row.channel_id, video_id).await {
            warn!(video_id = %video_id, error = %e, "failed to notify subscribers");
        }
    }

    info!(
        video_id = %video_id,
        duration_secs = duration as i32,
        size_bytes = total_size,
        variants = ladder.len(),
        "job done (ready)"
    );
    Ok((duration as i32, total_size))
}

/// Process a transcode job with retry.
///
/// Returns `Ok(())` on success (after up to `max_retries` attempts), or
/// `Err` after the final attempt — in which case the video row is marked
/// `failed` and a `transcode-failed` event is published.
pub async fn process(
    state: &WorkerState,
    queue: &JobQueue,
    job: &TranscodeJob,
) -> anyhow::Result<()> {
    let max = state.config.max_retries.max(1);
    let mut last_err: Option<String> = None;

    for attempt in 1..=max {
        info!(video_id = %job.video_id, attempt, max, "attempting job");
        match process_once(state, job).await {
            Ok(_) => {
                if attempt > 1 {
                    info!(video_id = %job.video_id, attempt, "succeeded on retry");
                }
                return Ok(());
            }
            Err(e) => {
                warn!(video_id = %job.video_id, attempt, max, error = %e, "attempt failed");
                last_err = Some(e.to_string());
                if attempt < max {
                    // Exponential backoff: 2s, 4s, 8s, ...
                    let backoff = Duration::from_secs(2u64.saturating_pow(attempt - 1));
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    // Final failure: mark the video failed, publish event.
    let err = last_err.expect("loop always sets last_err");
    if let Err(db_err) =
        videos::finalize(&state.db, job.video_id, VideoStatus::Failed, None, None, None, None, None).await
    {
        warn!(video_id = %job.video_id, error = %db_err, "failed to mark video failed in db");
    }
    let _ = queue
        .publish("transcode-failed", &format!("{job}: {err}"))
        .await;
    warn!(video_id = %job.video_id, error = %err, "job failed after {max} attempts");
    Err(anyhow::anyhow!("{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_probe_ok() {
        let json = r#"{ "format": { "duration": "42.5" }, "streams": [ { "codec_type": "video", "width": 1280, "height": 720 } ] }"#;
        assert_eq!(parse_probe(json).unwrap(), (42.5, 1280, 720));
    }

    #[test]
    fn parse_probe_picks_video_stream() {
        let json = r#"{ "format": { "duration": "1.0" }, "streams": [ { "codec_type": "audio" }, { "codec_type": "video", "width": 640, "height": 360 } ] }"#;
        assert_eq!(parse_probe(json).unwrap(), (1.0, 640, 360));
    }

    #[test]
    fn parse_probe_missing_duration() {
        let json = r#"{ "format": {}, "streams": [ { "codec_type": "video", "width": 1, "height": 1 } ] }"#;
        assert!(parse_probe(json).is_err());
    }

    #[test]
    fn parse_probe_missing_video_stream() {
        let json = r#"{ "format": { "duration": "1.0" }, "streams": [ { "codec_type": "audio" } ] }"#;
        assert!(parse_probe(json).is_err());
    }

    #[test]
    fn parse_probe_bad_duration() {
        let json = r#"{ "format": { "duration": "not-a-number" }, "streams": [ { "codec_type": "video", "width": 1, "height": 1 } ] }"#;
        assert!(parse_probe(json).is_err());
    }

    #[test]
    fn ladder_1080() {
        assert_eq!(quality_ladder(1080), vec![1080, 720, 480, 360]);
    }

    #[test]
    fn ladder_4k_caps_at_1080() {
        assert_eq!(quality_ladder(2160), vec![1080, 720, 480, 360]);
    }

    #[test]
    fn ladder_720() {
        assert_eq!(quality_ladder(720), vec![720, 480, 360]);
    }

    #[test]
    fn ladder_540_includes_native_top() {
        assert_eq!(quality_ladder(540), vec![540, 480, 360]);
    }

    #[test]
    fn ladder_240_single() {
        assert_eq!(quality_ladder(240), vec![240]);
    }
}
