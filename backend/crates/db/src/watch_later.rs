//! Watch-later query module.

use uuid::Uuid;

use crate::pool::Pool;
use crate::videos::VideoRow;

/// All of the user's watch-later videos, newest-saved first.
pub async fn list_for_user(pool: &Pool, user_id: Uuid) -> Result<Vec<VideoRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT v.* FROM watch_later wl
        JOIN videos v ON v.id = wl.video_id
        WHERE wl.user_id = $1
        ORDER BY wl.added_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Add a video to the user's watch-later list (idempotent).
pub async fn add(pool: &Pool, user_id: Uuid, video_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO watch_later (user_id, video_id) VALUES ($1, $2)
        ON CONFLICT (user_id, video_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a video from the user's watch-later list.
pub async fn remove(pool: &Pool, user_id: Uuid, video_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM watch_later WHERE user_id = $1 AND video_id = $2")
        .bind(user_id)
        .bind(video_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Whether the video is already in the user's watch-later list.
pub async fn contains(pool: &Pool, user_id: Uuid, video_id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM watch_later WHERE user_id = $1 AND video_id = $2")
            .bind(user_id)
            .bind(video_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some())
}
