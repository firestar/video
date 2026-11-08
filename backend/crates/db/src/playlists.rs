//! Playlist query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;
use domain::Playlist;

#[derive(Debug, Clone, FromRow)]
pub struct PlaylistRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub is_public: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(default)]
    pub video_count: i64,
}

impl From<PlaylistRow> for Playlist {
    fn from(r: PlaylistRow) -> Self {
        Playlist {
            id: r.id,
            user_id: r.user_id,
            title: r.title,
            description: r.description,
            is_public: r.is_public,
            created_at: r.created_at,
            updated_at: r.updated_at,
            video_count: r.video_count,
        }
    }
}

/// A playlist with its video count.
pub async fn create(
    pool: &Pool,
    user_id: Uuid,
    title: &str,
    description: &str,
    is_public: bool,
) -> Result<PlaylistRow, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO playlists (user_id, title, description, is_public)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(title)
    .bind(description)
    .bind(is_public)
    .fetch_one(pool)
    .await
}

/// Fetch one playlist with its video count.
pub async fn find_by_id(pool: &Pool, id: Uuid) -> Result<Option<PlaylistRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT p.*, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id)::bigint AS video_count
        FROM playlists p
        WHERE p.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// All playlists for a user, newest first, with video counts.
pub async fn list_for_user(pool: &Pool, user_id: Uuid) -> Result<Vec<PlaylistRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT p.*, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id)::bigint AS video_count
        FROM playlists p
        WHERE p.user_id = $1
        ORDER BY p.updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Update playlist fields (only provided ones).
pub async fn update(
    pool: &Pool,
    id: Uuid,
    user_id: Uuid,
    title: Option<&str>,
    description: Option<&str>,
    is_public: Option<bool>,
) -> Result<Option<PlaylistRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE playlists SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            is_public = COALESCE($5, is_public),
            updated_at = now()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(title)
    .bind(description)
    .bind(is_public)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &Pool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(r#"DELETE FROM playlists WHERE id = $1 AND user_id = $2"#)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Whether the user owns the playlist.
pub async fn is_owner(pool: &Pool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM playlists WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// Add a video to a playlist (owner only). Idempotent: returns `false` when the
/// video was already in the playlist. New items get position = max + 1.
pub async fn add_item(
    pool: &Pool,
    playlist_id: Uuid,
    user_id: Uuid,
    video_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        INSERT INTO playlist_items (playlist_id, video_id, position)
        SELECT $1, $3, COALESCE((SELECT MAX(position) FROM playlist_items WHERE playlist_id = $1), 0) + 1
        WHERE NOT EXISTS (
            SELECT 1 FROM playlist_items WHERE playlist_id = $1 AND video_id = $3
        )
          AND EXISTS (SELECT 1 FROM playlists WHERE id = $1 AND user_id = $2)
        "#,
    )
    .bind(playlist_id)
    .bind(user_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Remove a video from a playlist (owner only).
pub async fn remove_item(
    pool: &Pool,
    playlist_id: Uuid,
    user_id: Uuid,
    video_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        DELETE FROM playlist_items
        WHERE playlist_id = $1 AND video_id = $3
          AND EXISTS (SELECT 1 FROM playlists WHERE id = $1 AND user_id = $2)
        "#,
    )
    .bind(playlist_id)
    .bind(user_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}
/// Video ids in a playlist, in position order.
pub async fn video_ids(pool: &Pool, playlist_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        r#"SELECT video_id FROM playlist_items WHERE playlist_id = $1 ORDER BY position"#,
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await
}

/// Whether a video is in a playlist.
pub async fn has_item(
    pool: &Pool,
    playlist_id: Uuid,
    video_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<i32> = sqlx::query_scalar(
        r#"SELECT 1 FROM playlist_items WHERE playlist_id = $1 AND video_id = $2"#,
    )
    .bind(playlist_id)
    .bind(video_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
