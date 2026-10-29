//! Watch-history and view-record query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;
use domain::{ViewRecord, WatchHistory};

#[derive(Debug, FromRow)]
pub struct WatchHistoryRow {
    pub user_id: Uuid,
    pub video_id: Uuid,
    pub watched_until_secs: i32,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<WatchHistoryRow> for WatchHistory {
    fn from(r: WatchHistoryRow) -> Self {
        WatchHistory {
            user_id: r.user_id,
            video_id: r.video_id,
            watched_until_secs: r.watched_until_secs,
            updated_at: r.updated_at,
        }
    }
}

/// Upsert a watch-history row.
pub async fn upsert(
    pool: &Pool,
    user_id: Uuid,
    video_id: Uuid,
    watched_until_secs: i32,
) -> Result<WatchHistoryRow, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO watch_history (user_id, video_id, watched_until_secs, updated_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (user_id, video_id)
        DO UPDATE SET watched_until_secs = GREATEST(watched_history.watched_until_secs, EXCLUDED.watched_until_secs),
                      updated_at = now()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(video_id)
    .bind(watched_until_secs)
    .fetch_one(pool)
    .await
}

/// User's watch history, most-recently-watched first.
pub async fn list_for_user(
    pool: &Pool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<WatchHistoryRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT * FROM watch_history
        WHERE user_id = $1
        ORDER BY updated_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[derive(Debug, FromRow)]
pub struct ViewRecordRow {
    pub id: Uuid,
    pub video_id: Uuid,
    pub user_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ViewRecordRow> for ViewRecord {
    fn from(r: ViewRecordRow) -> Self {
        ViewRecord {
            id: r.id,
            video_id: r.video_id,
            user_id: r.user_id,
            created_at: r.created_at,
        }
    }
}

/// Record a view event and bump the video's view_count.
///
/// NOTE: the original used a single transaction; in this build sqlx 0.8.6
/// does not implement `Executor` for `&mut Transaction<'_, Postgres>`, so
/// the two statements run sequentially on the pool instead. The view count
/// increment is still an atomic `UPDATE ... SET view_count = view_count + 1`,
/// so concurrent viewers cannot lose counts; only strict two-statement
/// atomicity (no partial insert) is dropped.
pub async fn record_view(
    pool: &Pool,
    video_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<ViewRecordRow, sqlx::Error> {
    let row = sqlx::query_as(
        r#"
        INSERT INTO view_records (video_id, user_id)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(video_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    let _ = sqlx::query(r#"UPDATE videos SET view_count = view_count + 1 WHERE id = $1"#)
        .bind(video_id)
        .execute(pool)
        .await?;
    Ok(row)
}
