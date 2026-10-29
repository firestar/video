use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Processing state of a video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStatus {
    /// Upload received, awaiting transcode.
    Pending,
    /// FFmpeg job running.
    Processing,
    /// Ready to serve.
    Ready,
    /// Transcode failed.
    Failed,
}

/// A video.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub object_key: String,
    pub tags: Vec<String>,
    pub status: VideoStatus,
    pub duration_secs: Option<i32>,
    pub file_size: Option<i64>,
    pub view_count: i64,
    pub like_count: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Fields set when a video row is first created (upload started).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCreate {
    pub channel_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub object_key: String,
    pub tags: Vec<String>,
}

/// Fields updated by the worker after transcode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFinalize {
    pub status: VideoStatus,
    pub duration_secs: Option<i32>,
    pub file_size: Option<i64>,
    pub thumbnail_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_status_roundtrip() {
        for v in [
            VideoStatus::Pending,
            VideoStatus::Processing,
            VideoStatus::Ready,
            VideoStatus::Failed,
        ] {
            let s = serde_json::to_string(&v).unwrap();
            let b: VideoStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(v, b);
        }
    }

    #[test]
    fn video_serializes() {
        let v = Video {
            id: Uuid::new_v4(),
            channel_id: Uuid::new_v4(),
            title: "T".into(),
            description: None,
            thumbnail_url: None,
            object_key: "videos/x".into(),
            tags: vec!["a".into()],
            status: VideoStatus::Pending,
            duration_secs: None,
            file_size: None,
            view_count: 0,
            like_count: 0,
            published_at: None,
            created_at: Utc::now(),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"channelId\""));
        assert!(s.contains("\"viewCount\""));
        assert!(s.contains("\"objectKey\""));
        assert!(s.contains("\"tags\""));
    }
}
