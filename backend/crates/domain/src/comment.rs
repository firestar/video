use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A comment on a video.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: Uuid,
    pub video_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub body: String,
    pub like_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Threaded comment node: a comment with its replies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentNode {
    pub comment: Comment,
    pub replies: Vec<Comment>,
}

/// Request body for POST /api/videos/{id}/comments.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentCreate {
    pub body: String,
    pub parent_id: Option<Uuid>,
}

impl CommentCreate {
    pub fn validate(&self) -> Result<(), String> {
        let t = self.body.trim();
        if t.is_empty() {
            return Err("comment body required".into());
        }
        if t.len() > 10_000 {
            return Err("comment too long (max 10000 chars)".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_create_validates() {
        assert!(CommentCreate { body: "hi".into(), parent_id: None }.validate().is_ok());
        assert!(CommentCreate { body: "".into(), parent_id: None }.validate().is_err());
        assert!(CommentCreate {
            body: "x".repeat(10_001),
            parent_id: None
        }
        .validate()
        .is_err());
    }
}
