//! Watch-history and view-record types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A watch-history record (one per user+video, upserted on seek).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchHistory {
    pub user_id: Uuid,
    pub video_id: Uuid,
    pub watched_until_secs: i32,
    pub updated_at: DateTime<Utc>,
}

/// A single view event for a video (one row per play session).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRecord {
    pub id: Uuid,
    pub video_id: Uuid,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Query params for GET /api/history.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_history_serializes() {
        let w = WatchHistory {
            user_id: Uuid::new_v4(),
            video_id: Uuid::new_v4(),
            watched_until_secs: 42,
            updated_at: Utc::now(),
        };
        let j = serde_json::to_string(&w).unwrap();
        assert!(j.contains("\"watchedUntilSecs\""));
    }

    #[test]
    fn view_record_serializes() {
        let v = ViewRecord {
            id: Uuid::new_v4(),
            video_id: Uuid::new_v4(),
            user_id: None,
            created_at: Utc::now(),
        };
        let j = serde_json::to_string(&v).unwrap();
        assert!(j.contains("\"videoId\""));
    }

    #[test]
    fn history_query_deserializes() {
        let q: HistoryQuery = serde_json::from_str(r#"{"limit":10}"#).unwrap();
        assert_eq!(q.limit, Some(10));
        assert_eq!(q.offset, None);
    }
}
