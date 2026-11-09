//! Startup recovery: re-enqueue videos stuck in `processing`.
//!
//! A job is BRPOP'd (consumed) from the Redis list the moment a worker picks
//! it up. If that worker instance dies mid-pipeline, the message is gone and
//! the video row stays `processing` forever. On startup the worker scans for
//! such rows (db::videos::find_processing) and re-enqueues them; the
//! pop-time status guard in `jobs::process` makes duplicates harmless (e.g.
//! a job that was still sitting in the Redis list when the worker died).

use db::videos::ProcessingVideo;

use crate::queue::TranscodeJob;

/// Build the recovery queue jobs for the given stuck rows.
pub fn jobs(rows: &[ProcessingVideo]) -> Vec<TranscodeJob> {
    rows.iter()
        .map(|r| TranscodeJob {
            video_id: r.id,
            object_key: r.object_key.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::parse_job;
    use uuid::Uuid;

    fn stuck_row(id: Uuid, key: &str) -> ProcessingVideo {
        ProcessingVideo {
            id,
            object_key: key.to_string(),
        }
    }

    #[test]
    fn maps_stuck_rows_to_jobs() {
        let id = Uuid::new_v4();
        let rows = vec![stuck_row(id, "videos/src.mp4")];
        let js = jobs(&rows);
        assert_eq!(js.len(), 1);
        assert_eq!(js[0].video_id, id);
        assert_eq!(js[0].object_key, "videos/src.mp4");
    }

    #[test]
    fn empty_rows_yield_no_jobs() {
        assert!(jobs(&[]).is_empty());
    }

    #[test]
    fn recovery_payload_round_trips_through_pop() {
        // The worker's BRPOP path parses payloads with parse_job; recovery
        // jobs must serialize to a shape it accepts.
        let id = Uuid::new_v4();
        let job = TranscodeJob {
            video_id: id,
            object_key: "videos/abc".into(),
        };
        let payload = serde_json::to_string(&job).unwrap();
        let parsed = parse_job(&payload).unwrap();
        assert_eq!(parsed.video_id, job.video_id);
        assert_eq!(parsed.object_key, job.object_key);
    }

    #[test]
    fn recovery_payload_matches_api_shape() {
        // The API pushes {"video_id": ..., "object_key": ...}; recovery must
        // emit exactly the same keys so both paths interoperate.
        let id = Uuid::new_v4();
        let job = TranscodeJob {
            video_id: id,
            object_key: "videos/abc".into(),
        };
        let v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(v["video_id"], id.to_string());
        assert_eq!(v["object_key"], "videos/abc");
    }
}
