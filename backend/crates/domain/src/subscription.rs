//! Subscription entity types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A channel subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub subscriber_id: Uuid,
    pub channel_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Request body for POST /api/subscriptions.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    pub channel_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_serializes() {
        let s = Subscription {
            subscriber_id: Uuid::new_v4(),
            channel_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"subscriberId\""));
    }

    #[test]
    fn subscribe_request_deserializes() {
        let id = Uuid::new_v4();
        let j = format!("{{\"channelId\":\"{}\"}}", id);
        let r: SubscribeRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(r.channel_id, id);
    }
}
