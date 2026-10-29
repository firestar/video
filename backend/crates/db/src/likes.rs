//! Like query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;
use domain::Like;

#[derive(Debug, FromRow)]
pub struct LikeRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub video_id: Uuid,
    pub comment_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<LikeRow> for Like {
    fn from(r: LikeRow) -> Self {
        Like {
            id: r.id,
            user_id: r.user_id,
            video_id: r.video_id,
            comment_id: r.comment_id,
            created_at: r.created_at,
        }
    }
}

/// Like a video (or a comment if comment_id is Some).
pub async fn like(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    comment_id: Option<Uuid>,
) -> Result<Option<LikeRow>, sqlx::Error> {
    let row = sqlx::query_as(
        r#"
        INSERT INTO likes (user_id, video_id, comment_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(video_id)
    .bind(comment_id)
    .fetch_optional(pool)
    .await?;

    if row.is_some() {
        // Bump the counter on the target.
        if let Some(cid) = comment_id {
            let _ = sqlx::query(r#"UPDATE comments SET like_count = like_count + 1 WHERE id = $1"#)
                .bind(cid)
                .execute(pool)
                .await;
        } else {
            let _ = sqlx::query(r#"UPDATE videos SET like_count = like_count + 1 WHERE id = $1"#)
                .bind(video_id)
                .execute(pool)
                .await;
        }
    }
    Ok(row)
}

/// Unlike a video/comment.
pub async fn unlike(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    comment_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"DELETE FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id = $3"#,
    )
    .bind(user_id)
    .bind(video_id)
    .bind(comment_id)
    .execute(pool)
    .await?;
    if res.rows_affected() > 0 {
        if let Some(cid) = comment_id {
            let _ = sqlx::query(
                r#"UPDATE comments SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1"#,
            )
            .bind(cid)
            .execute(pool)
            .await;
        } else {
            let _ = sqlx::query(
                r#"UPDATE videos SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1"#,
            )
            .bind(video_id)
            .execute(pool)
            .await;
        }
    }
    Ok(res.rows_affected() > 0)
}

/// Whether a user liked a video.
pub async fn has_liked(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL"#,
    )
    .bind(user_id)
    .bind(video_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
