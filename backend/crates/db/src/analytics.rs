//! Channel analytics aggregation queries (Studio dashboard).

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;

/// One day's count in a time-series series.
#[derive(Debug, FromRow, serde::Serialize)]
pub struct DayCount {
    #[sqlx(rename = "day")]
    pub day: String,
    #[sqlx(rename = "count")]
    pub count: i64,
}

/// A top video row for the analytics dashboard.
#[derive(Debug, FromRow, serde::Serialize)]
pub struct TopVideoRow {
    pub id: Uuid,
    pub title: String,
    #[sqlx(rename = "thumbnail_url")]
    pub thumbnail_url: Option<String>,
    #[sqlx(rename = "view_count")]
    pub view_count: i64,
    #[sqlx(rename = "like_count")]
    pub like_count: i64,
}

/// Total views across all of a channel's videos.
pub async fn total_views(pool: &Pool, channel_id: Uuid) -> Result<i64, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        r#"SELECT COALESCE(SUM(view_count), 0)::bigint FROM videos WHERE channel_id = $1"#,
    )
    .bind(channel_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Total subscribers for a channel.
pub async fn total_subscribers(pool: &Pool, channel_id: Uuid) -> Result<i64, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM subscriptions WHERE channel_id = $1"#,
    )
    .bind(channel_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Daily view counts for a channel's videos over the last `days` days.
///
/// Returns one row per day that has at least one view (the client fills in
/// missing days with zero). Days are calendar days in UTC, labelled YYYY-MM-DD.
pub async fn views_by_day(
    pool: &Pool,
    channel_id: Uuid,
    days: i32,
) -> Result<Vec<DayCount>, sqlx::Error> {
    let rows = sqlx::query_as(
        r#"
        SELECT to_char(date_trunc('day', vr.created_at), 'YYYY-MM-DD') AS day,
               COUNT(*)::bigint AS count
        FROM view_records vr
        JOIN videos v ON v.id = vr.video_id
        WHERE v.channel_id = $1
          AND vr.created_at >= now() - make_interval(days => $2)
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(channel_id)
    .bind(days)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Daily new-subscriber counts for a channel over the last `days` days.
pub async fn subscribers_by_day(
    pool: &Pool,
    channel_id: Uuid,
    days: i32,
) -> Result<Vec<DayCount>, sqlx::Error> {
    let rows = sqlx::query_as(
        r#"
        SELECT to_char(date_trunc('day', created_at), 'YYYY-MM-DD') AS day,
               COUNT(*)::bigint AS count
        FROM subscriptions
        WHERE channel_id = $1
          AND created_at >= now() - make_interval(days => $2)
        GROUP BY 1
        ORDER BY 1
        "#,
    )
    .bind(channel_id)
    .bind(days)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// A channel's top videos by view count (ready videos only).
pub async fn top_videos(
    pool: &Pool,
    channel_id: Uuid,
    limit: i64,
) -> Result<Vec<TopVideoRow>, sqlx::Error> {
    let rows = sqlx::query_as(
        r#"
        SELECT id, title, thumbnail_url, view_count, like_count
        FROM videos
        WHERE channel_id = $1 AND status = 'ready'
        ORDER BY view_count DESC, created_at DESC
        LIMIT $2
        "#,
    )
    .bind(channel_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// The start of the UTC day `days` days ago (inclusive), for the client to
/// build a contiguous day axis. Exposed for tests.
#[allow(dead_code)]
pub fn window_start(days: i32) -> DateTime<Utc> {
    Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
        - chrono::Duration::days((days - 1) as i64)
}
