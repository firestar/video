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

/// Parse `ffprobe -v quiet -print_format json -show_format -i <file>` output
/// into the container duration in seconds.
fn parse_duration(probe_stdout: &str) -> anyhow::Result<f64> {
    let v: Value = serde_json::from_str(probe_stdout)
        .map_err(|e| anyhow::anyhow!("ffprobe JSON parse: {e}"))?;
    let dur = v
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .ok_or_else(|| anyhow::anyhow!("ffprobe output missing format.duration"))?;
    dur.parse::<f64>()
        .map_err(|e| anyhow::anyhow!("ffprobe duration parse: {e}"))
}

/// Run ffprobe on `input`, return the duration in seconds.
async fn probe_duration(ffprobe: &str, input: &std::path::Path) -> anyhow::Result<f64> {
    let out = Command::new(ffprobe)
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
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
    parse_duration(&String::from_utf8_lossy(&out.stdout))
}

/// Run ffmpeg transcode: H.264 + AAC, 1080p, faststart.
async fn transcode(ffmpeg: &str, input: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    let out = Command::new(ffmpeg)
        .arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("medium")
        .arg("-crf")
        .arg("23")
        .arg("-vf")
        .arg("scale=-2:1080")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("ffmpeg spawn: {e}"))?;
    if !out.status.success() {
        bail!(
            "ffmpeg transcode failed ({}): {}",
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

/// One full pipeline attempt. Returns `(duration_secs, output_size_bytes)`.
async fn process_once(state: &WorkerState, job: &TranscodeJob) -> anyhow::Result<(i32, i64)> {
    let video_id = job.video_id;
    info!(video_id = %video_id, object_key = %job.object_key, "processing (attempt)");

    // 1. Download source from R2 to a temp file.
    let tmp = TempDir::new().map_err(|e| anyhow::anyhow!("tempdir: {e}"))?;
    let src_path = tmp.path().join("source");
    let out_path = tmp.path().join(format!("{video_id}.mp4"));
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

    // 2. ffprobe → duration.
    let duration = probe_duration(&state.config.ffprobe_path, &src_path).await?;
    info!(video_id = %video_id, duration_secs = duration, "probed");

    // 3. Transcode.
    transcode(&state.config.ffmpeg_path, &src_path, &out_path).await?;

    // 4. Thumbnail at 25% of duration.
    thumbnail(&state.config.ffmpeg_path, &src_path, duration, &thumb_path).await?;

    // 5. Upload transcode to R2.
    let out_key = format!("videos/{video_id}.mp4");
    let out_bytes = fs::read(&out_path).await.map_err(|e| anyhow::anyhow!("read output: {e}"))?;
    state
        .storage
        .upload(&out_key, out_bytes.clone().into(), "video/mp4")
        .await
        .map_err(|e| anyhow::anyhow!("upload output: {e}"))?;

    // 6. Upload thumbnail to R2.
    let thumb_key = format!("thumbnails/{video_id}.jpg");
    let thumb_bytes = fs::read(&thumb_path).await.map_err(|e| anyhow::anyhow!("read thumbnail: {e}"))?;
    state
        .storage
        .upload(&thumb_key, thumb_bytes.into(), "image/jpeg")
        .await
        .map_err(|e| anyhow::anyhow!("upload thumbnail: {e}"))?;

    // 7. Build public URLs.
    let _video_url = state.storage.public_url(&out_key);
    let thumbnail_url = state.storage.public_url(&thumb_key);

    // 8. Finalize DB row.
    videos::finalize(
        &state.db,
        video_id,
        VideoStatus::Ready,
        Some(duration as i32),
        Some(out_bytes.len() as i64),
        thumbnail_url.as_deref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("db finalize: {e}"))?;

    info!(
        video_id = %video_id,
        duration_secs = duration as i32,
        size_bytes = out_bytes.len() as i64,
        "job done (ready)"
    );
    Ok((duration as i32, out_bytes.len() as i64))
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
        videos::finalize(&state.db, job.video_id, VideoStatus::Failed, None, None, None).await
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
    fn parse_duration_ok() {
        let json = r#"{ "format": { "duration": "42.5" } }"#;
        assert_eq!(parse_duration(json).unwrap(), 42.5);
    }

    #[test]
    fn parse_duration_missing() {
        let json = r#"{ "format": {} }"#;
        assert!(parse_duration(json).is_err());
    }

    #[test]
    fn parse_duration_bad() {
        let json = r#"{ "format": { "duration": "not-a-number" } }"#;
        assert!(parse_duration(json).is_err());
    }
}
