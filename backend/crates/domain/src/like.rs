//! Like entity types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A like on a video or a comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Like {
    pub id: Uuid,
    pub user_id: Uuid,
    pub video_id: Uuid,
    pub comment_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_serializes() {
        let l = Like {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            video_id: Uuid::new_v4(),
            comment_id: None,
            created_at: Utc::now(),
        };
        let j = serde_json::to_string(&l).unwrap();
        assert!(j.contains("\"commentId\":null"));
    }
}
