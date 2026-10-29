use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A channel (owned by a user).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub handle: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub banner_url: Option<String>,
}

/// GET /api/channels/{id} response: channel info + live subscription count.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub channel: Channel,
    pub subscriber_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_serializes() {
        let c = Channel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test".into(),
            handle: "test".into(),
            description: None,
            banner_url: None,
            created_at: Utc::now(),
        };
        let s = serde_json::to_string(&c).unwrap();
        assert!(s.contains("\"userId\""));
    }
}
