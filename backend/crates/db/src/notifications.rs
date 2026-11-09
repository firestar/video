//! Notification query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;

/// One notification row joined with the channel + video that caused it.
#[derive(Debug, FromRow)]
pub struct NotificationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub video_id: Option<Uuid>,
    pub r#type: String,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub channel_name: String,
    pub channel_avatar_url: Option<String>,
    pub video_title: Option<String>,
    pub video_thumbnail_url: Option<String>,
}

/// Fan out a `new_upload` notification to every subscriber of `channel_id`,
/// excluding the channel owner. Returns the number of rows inserted.
pub async fn notify_new_uploads(
    pool: &Pool,
    channel_id: Uuid,
    video_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let res = sqlx::query(
        r#"
        INSERT INTO notifications (user_id, channel_id, video_id, type)
        SELECT s.subscriber_id, $1, $2, 'new_upload'
        FROM subscriptions s
        WHERE s.channel_id = $1
          AND s.subscriber_id != (SELECT user_id FROM channels WHERE id = $1)
        "#,
    )
    .bind(channel_id)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() as i64)
}
/// Notify the video's channel owner that someone commented on their video.
/// No-op when the commenter is the owner. Returns rows inserted.
pub async fn notify_new_comment(
    pool: &Pool,
    video_id: Uuid,
    commenter_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let res = sqlx::query(
        r#"
         INSERT INTO notifications (user_id, channel_id, video_id, type)
         SELECT c.user_id, ac.id, $1, 'new_comment'
         FROM videos v
         JOIN channels c ON c.id = v.channel_id
         JOIN channels ac ON ac.user_id = $2
         WHERE v.id = $1
           AND c.user_id != $2
         "#,
    )
    .bind(video_id)
    .bind(commenter_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() as i64)
}

/// Notify the video's channel owner that someone liked their video.
/// No-op when the liker is the owner. Returns rows inserted.
pub async fn notify_new_like(
    pool: &Pool,
    video_id: Uuid,
    liker_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let res = sqlx::query(
        r#"
         INSERT INTO notifications (user_id, channel_id, video_id, type)
         SELECT c.user_id, ac.id, $1, 'new_like'
         FROM videos v
         JOIN channels c ON c.id = v.channel_id
         JOIN channels ac ON ac.user_id = $2
         WHERE v.id = $1
           AND c.user_id != $2
        "#,
    )
    .bind(video_id)
    .bind(liker_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() as i64)
}


/// Latest notifications for a user (newest first), with channel + video context.
pub async fn list_for_user(
    pool: &Pool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<NotificationRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT n.id, n.user_id, n.channel_id, n.video_id, n.type, n.read_at, n.created_at,
               c.name AS channel_name,
               u.avatar_url AS channel_avatar_url,
               v.title AS video_title,
               v.thumbnail_url AS video_thumbnail_url
        FROM notifications n
        JOIN channels c ON c.id = n.channel_id
        JOIN users u ON u.id = c.user_id
        LEFT JOIN videos v ON v.id = n.video_id
        WHERE n.user_id = $1
        ORDER BY n.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Count of unread notifications for a user.
pub async fn unread_count(pool: &Pool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Mark one notification read (only if it belongs to the user and is unread).
/// Returns true when a row was updated.
pub async fn mark_read(pool: &Pool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"UPDATE notifications SET read_at = now()
        WHERE id = $1 AND user_id = $2 AND read_at IS NULL"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Mark every notification for a user read. Returns rows updated.
pub async fn mark_all_read(pool: &Pool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let res = sqlx::query(
        r#"UPDATE notifications SET read_at = now()
        WHERE user_id = $1 AND read_at IS NULL"#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() as i64)
}
