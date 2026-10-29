//! Video query module.

use uuid::Uuid;

use crate::pool::Pool;
use domain::{Video, VideoCreate, VideoStatus};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct VideoRow {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub object_key: String,
    pub tags: Vec<String>,
    pub status: String,
    pub duration_secs: Option<i32>,
    pub file_size: Option<i64>,
    pub view_count: i64,
    pub like_count: i64,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn status_to_enum(s: &str) -> VideoStatus {
    match s {
        "processing" => VideoStatus::Processing,
        "ready" => VideoStatus::Ready,
        "failed" => VideoStatus::Failed,
        _ => VideoStatus::Pending,
    }
}

fn enum_to_str(s: VideoStatus) -> &'static str {
    match s {
        VideoStatus::Pending => "pending",
        VideoStatus::Processing => "processing",
        VideoStatus::Ready => "ready",
        VideoStatus::Failed => "failed",
    }
}

impl From<VideoRow> for Video {
    fn from(r: VideoRow) -> Self {
        Video {
            id: r.id,
            channel_id: r.channel_id,
            title: r.title,
            description: r.description,
            thumbnail_url: r.thumbnail_url,
            object_key: r.object_key,
            tags: r.tags,
            status: status_to_enum(&r.status),
            duration_secs: r.duration_secs,
            file_size: r.file_size,
            view_count: r.view_count,
            like_count: r.like_count,
            published_at: r.published_at,
            created_at: r.created_at,
        }
    }
}

/// Create a processing video row (upload completed, awaiting transcode).
pub async fn create(pool: &Pool, req: &VideoCreate) -> Result<VideoRow, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO videos (channel_id, title, description, object_key, tags, status)
        VALUES ($1, $2, $3, $4, $5, 'processing')
        RETURNING *
        "#,
    )
    .bind(req.channel_id)
    .bind(req.title.as_str())
    .bind(req.description.as_deref())
    .bind(req.object_key.as_str())
    .bind(&req.tags)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &Pool, id: Uuid) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as(r#"SELECT * FROM videos WHERE id = $1"#)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Worker finalization: set status, duration, size, thumbnail, publish time.
pub async fn finalize(
    pool: &Pool,
    id: Uuid,
    status: VideoStatus,
    duration_secs: Option<i32>,
    file_size: Option<i64>,
    thumbnail_url: Option<&str>,
) -> Result<Option<VideoRow>, sqlx::Error> {
    let status_str = enum_to_str(status);
    sqlx::query_as(
        r#"
        UPDATE videos SET
            status = $2,
            duration_secs = $3,
            file_size = $4,
            thumbnail_url = COALESCE($5, thumbnail_url),
            published_at = CASE WHEN $2 = 'ready' AND published_at IS NULL
                THEN now() ELSE published_at END
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(status_str)
    .bind(duration_secs)
    .bind(file_size)
    .bind(thumbnail_url)
    .fetch_optional(pool)
    .await
}

/// Increment view count (atomic).
pub async fn increment_views(pool: &Pool, id: Uuid) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE videos SET view_count = view_count + 1 WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Ordering for public video listings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VideoSort {
    /// Newest first (default).
    #[default]
    Newest,
    /// Trending: most views/likes, newest as tiebreak.
    Trending,
}

impl VideoSort {
    /// Parse a query-string value ("newest" | "trending"); unknown -> default.
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "trending" => Self::Trending,
            _ => Self::Newest,
        }
    }
}

/// Public listing: ready videos, paginated, ordered by `sort`.
pub async fn list_public(
    pool: &Pool,
    limit: i64,
    offset: i64,
    sort: VideoSort,
) -> Result<Vec<VideoRow>, sqlx::Error> {
    let order_by = match sort {
        VideoSort::Newest => {
            "published_at DESC NULLS LAST, created_at DESC"
        }
        VideoSort::Trending => {
            "view_count DESC, like_count DESC, published_at DESC NULLS LAST, created_at DESC"
        }
    };
    let sql = format!(
        r#"SELECT * FROM videos
        WHERE status = 'ready'
        ORDER BY {order_by}
        LIMIT $1 OFFSET $2"#
    );
    sqlx::query_as(sql.as_str())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Ready videos for one channel.
pub async fn list_for_channel(
    pool: &Pool,
    channel_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<VideoRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT * FROM videos
        WHERE channel_id = $1 AND status = 'ready'
        ORDER BY published_at DESC NULLS LAST, created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(channel_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Full-text search over title/description.
pub async fn search(pool: &Pool, query: &str, limit: i64) -> Result<Vec<VideoRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT * FROM videos
        WHERE status = 'ready'
          AND search_vector @@ plainto_tsquery('simple', $1)
        ORDER BY ts_rank(search_vector, plainto_tsquery('simple', $1)) DESC
        LIMIT $2
        "#,
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await
}
