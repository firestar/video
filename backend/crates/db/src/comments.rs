//! Comment query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;
use domain::{Comment, CommentCreate};

#[derive(Debug, FromRow)]
pub struct CommentRow {
    pub id: Uuid,
    pub video_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub body: String,
    pub like_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<CommentRow> for Comment {
    fn from(r: CommentRow) -> Self {
        Comment {
            id: r.id,
            video_id: r.video_id,
            user_id: r.user_id,
            parent_id: r.parent_id,
            body: r.body,
            like_count: r.like_count,
            created_at: r.created_at,
        }
    }
}
pub async fn create(
    pool: &Pool,
    video_id: Uuid,
    user_id: Uuid,
    req: &CommentCreate,
) -> Result<CommentRow, domain::CommentError> {
    // If this is a reply, ensure the parent exists on this video and is top-level.
    if let Some(parent_id) = req.parent_id {
        let parent = find_by_id(pool, parent_id)
            .await
            .map_err(|_| domain::CommentError::ParentNotFound)?;
        let parent = parent.ok_or(domain::CommentError::ParentNotFound)?;
        if parent.video_id != video_id {
            return Err(domain::CommentError::WrongVideo);
        }
        if parent.parent_id.is_some() {
            return Err(domain::CommentError::NotTopLevel);
        }
    }
    let row = sqlx::query_as(
        r#"
        INSERT INTO comments (video_id, user_id, body, parent_id)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(video_id)
    .bind(user_id)
    .bind(&req.body)
    .bind(req.parent_id)
    .fetch_one(pool)
    .await
    .map_err(|e| domain::CommentError::Db(e.to_string()))?;
    Ok(row)
}

/// Fetch a single comment by id.
pub async fn find_by_id(
    pool: &Pool,
    id: Uuid,
) -> Result<Option<CommentRow>, sqlx::Error> {
    sqlx::query_as(r#"SELECT * FROM comments WHERE id = $1"#)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Threaded list: top-level comments each with their replies (1-level).
pub async fn list_for_video_threaded(
    pool: &Pool,
    video_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<domain::CommentNode>, sqlx::Error> {
    let rows: Vec<CommentRow> = sqlx::query_as(
        r#"
        SELECT * FROM comments
        WHERE video_id = $1
        ORDER BY created_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(video_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    let comments: Vec<domain::Comment> = rows.into_iter().map(Into::into).collect();

    // Separate top-level and replies, preserving order.
    let mut nodes: Vec<domain::CommentNode> = Vec::new();
    let mut reply_map: std::collections::HashMap<Uuid, Vec<domain::Comment>> =
        std::collections::HashMap::new();
    for c in comments {
        match c.parent_id {
            Some(pid) => reply_map.entry(pid).or_default().push(c),
            None => nodes.push(domain::CommentNode {
                comment: c,
                replies: Vec::new(),
            }),
        }
    }
    for node in &mut nodes {
        let replies = reply_map
            .remove(&node.comment.id)
            .unwrap_or_default();
        node.replies = replies;
    }
    Ok(nodes)
}

/// Delete a comment: returns true if the row was removed. Caller checks
/// author vs owner before calling. FK cascades replies.
pub async fn delete(pool: &Pool, id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(r#"DELETE FROM comments WHERE id = $1"#)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
