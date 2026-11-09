//! Like query module.
//!
//! `kind` distinguishes video likes from dislikes: they are mutually
//! exclusive (liking removes a dislike and vice versa). Comments are
//! like-only — `kind` is always 'like' for comment rows.

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
    pub kind: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<LikeRow> for Like {
    fn from(r: LikeRow) -> Self {
        Like {
            id: r.id,
            user_id: r.user_id,
            video_id: r.video_id,
            comment_id: r.comment_id,
            kind: r.kind,
            created_at: r.created_at,
        }
    }
}

/// Like a video (or a comment if comment_id is Some).
/// Liking a video removes an existing dislike (mutual exclusion).
pub async fn like(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    comment_id: Option<Uuid>,
) -> Result<Option<LikeRow>, sqlx::Error> {
    // Mutual exclusion: liking a video drops a prior dislike.
    if comment_id.is_none() {
        let removed = sqlx::query(
            r#"DELETE FROM likes
               WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL AND kind = 'dislike'"#,
        )
        .bind(user_id)
        .bind(video_id)
        .execute(pool)
        .await?;
        if removed.rows_affected() > 0 {
            let _ = sqlx::query(
                r#"UPDATE videos SET dislike_count = GREATEST(dislike_count - 1, 0) WHERE id = $1"#,
            )
            .bind(video_id)
            .execute(pool)
            .await;
        }
    }

    let row = sqlx::query_as(
        r#"
        INSERT INTO likes (user_id, video_id, comment_id, kind)
        VALUES ($1, $2, $3, 'like')
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

/// Dislike a video. Disliking removes an existing like (mutual exclusion).
pub async fn dislike(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
) -> Result<Option<LikeRow>, sqlx::Error> {
    let removed = sqlx::query(
        r#"DELETE FROM likes
           WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL AND kind = 'like'"#,
    )
    .bind(user_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    if removed.rows_affected() > 0 {
        let _ = sqlx::query(
            r#"UPDATE videos SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1"#,
        )
        .bind(video_id)
        .execute(pool)
        .await;
    }

    let row = sqlx::query_as(
        r#"
        INSERT INTO likes (user_id, video_id, comment_id, kind)
        VALUES ($1, $2, NULL, 'dislike')
        ON CONFLICT DO NOTHING
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(video_id)
    .fetch_optional(pool)
    .await?;

    if row.is_some() {
        let _ = sqlx::query(r#"UPDATE videos SET dislike_count = dislike_count + 1 WHERE id = $1"#)
            .bind(video_id)
            .execute(pool)
            .await;
    }
    Ok(row)
}

/// Unlike a video/comment (like rows only).
pub async fn unlike(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    comment_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"DELETE FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id IS NOT DISTINCT FROM $3 AND kind = 'like'"#,
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

/// Undislike a video.
pub async fn undislike(pool: &Pool, user_id: Uuid, video_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"DELETE FROM likes
           WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL AND kind = 'dislike'"#,
    )
    .bind(user_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    if res.rows_affected() > 0 {
        let _ = sqlx::query(
            r#"UPDATE videos SET dislike_count = GREATEST(dislike_count - 1, 0) WHERE id = $1"#,
        )
        .bind(video_id)
        .execute(pool)
        .await;
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
        r#"SELECT 1 FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL AND kind = 'like'"#,
    )
    .bind(user_id)
    .bind(video_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// Whether a user disliked a video.
pub async fn has_disliked(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id IS NULL AND kind = 'dislike'"#,
    )
    .bind(user_id)
    .bind(video_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

pub async fn has_comment_liked(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    comment_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM likes WHERE user_id = $1 AND video_id = $2 AND comment_id = $3 AND kind = 'like'"#,
    )
    .bind(user_id)
    .bind(video_id)
    .bind(comment_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
