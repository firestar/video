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
) -> Result<CommentRow, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO comments (video_id, user_id, body)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(video_id)
    .bind(user_id)
    .bind(&req.body)
    .fetch_one(pool)
    .await
}

pub async fn list_for_video(
    pool: &Pool,
    video_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<CommentRow>, sqlx::Error> {
    sqlx::query_as(
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
    .await
}

pub async fn delete(pool: &Pool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(r#"DELETE FROM comments WHERE id = $1 AND user_id = $2"#)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
