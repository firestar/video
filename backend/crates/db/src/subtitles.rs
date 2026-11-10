//! Subtitles query module: per-video caption tracks (one row per language).

use uuid::Uuid;

use crate::pool::Pool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubtitleRow {
    pub id: Uuid,
    pub video_id: Uuid,
    pub language: String,
    pub format: String,
    pub object_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Insert or replace the track for (video_id, language). Re-uploading a
/// language overwrites the previous object key.
pub async fn upsert(
    pool: &Pool,
    video_id: Uuid,
    language: &str,
    format: &str,
    object_key: &str,
) -> Result<SubtitleRow, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO subtitles (video_id, language, format, object_key)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (video_id, language)
        DO UPDATE SET format = EXCLUDED.format,
                      object_key = EXCLUDED.object_key
        RETURNING *
        "#,
    )
    .bind(video_id)
    .bind(language)
    .bind(format)
    .bind(object_key)
    .fetch_one(pool)
    .await
}

/// All tracks for a video, ordered by language.
pub async fn list_for_video(pool: &Pool, video_id: Uuid) -> Result<Vec<SubtitleRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM subtitles WHERE video_id = $1 ORDER BY language")
        .bind(video_id)
        .fetch_all(pool)
        .await
}

/// Delete one track; returns the removed row (for its storage object key).
pub async fn delete(
    pool: &Pool,
    video_id: Uuid,
    language: &str,
) -> Result<Option<SubtitleRow>, sqlx::Error> {
    sqlx::query_as("DELETE FROM subtitles WHERE video_id = $1 AND language = $2 RETURNING *")
        .bind(video_id)
        .bind(language)
        .fetch_optional(pool)
        .await
}

/// All tracks for a video (used when deleting the video itself).
pub async fn delete_for_video(pool: &Pool, video_id: Uuid) -> Result<Vec<SubtitleRow>, sqlx::Error> {
    sqlx::query_as("DELETE FROM subtitles WHERE video_id = $1 RETURNING *")
        .bind(video_id)
        .fetch_all(pool)
        .await
}
