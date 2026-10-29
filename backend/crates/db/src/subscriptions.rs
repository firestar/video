//! Subscription query module.

use sqlx::FromRow;
use uuid::Uuid;

use crate::pool::Pool;
use domain::Subscription;

#[derive(Debug, FromRow)]
pub struct SubscriptionRow {
    pub subscriber_id: Uuid,
    pub channel_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<SubscriptionRow> for Subscription {
    fn from(r: SubscriptionRow) -> Self {
        Subscription {
            subscriber_id: r.subscriber_id,
            channel_id: r.channel_id,
            created_at: r.created_at,
        }
    }
}

/// Subscribe a user to a channel. Idempotent: returns `false` when the
/// subscription already existed (no error), `true` when newly created.
pub async fn subscribe(
    pool: &Pool,
    subscriber_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"
        INSERT INTO subscriptions (subscriber_id, channel_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(subscriber_id)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}
pub async fn unsubscribe(
    pool: &Pool,
    subscriber_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"DELETE FROM subscriptions WHERE subscriber_id = $1 AND channel_id = $2"#,
    )
    .bind(subscriber_id)
    .bind(channel_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn is_subscribed(
    pool: &Pool,
    subscriber_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"SELECT 1 FROM subscriptions WHERE subscriber_id = $1 AND channel_id = $2"#,
    )
    .bind(subscriber_id)
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// Channels a user is subscribed to.
pub async fn list_channels_for_user(
    pool: &Pool,
    subscriber_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT channel_id FROM subscriptions WHERE subscriber_id = $1"#,
    )
    .bind(subscriber_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}


/// Subscription count for one channel.
pub async fn count_for_channel(
    pool: &Pool,
    channel_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let n = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM subscriptions WHERE channel_id = $1"#,
    )
    .bind(channel_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Subscriptions' channel rows, ordered by most recent subscription.
pub async fn channels_for_user(
    pool: &Pool,
    subscriber_id: Uuid,
) -> Result<Vec<crate::channels::ChannelRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT c.* FROM subscriptions s
        JOIN channels c ON c.id = s.channel_id
        WHERE s.subscriber_id = $1
        ORDER BY s.created_at DESC"#,
    )
    .bind(subscriber_id)
    .fetch_all(pool)
    .await?;
    rows
        .into_iter()
        .map(crate::channels::row_to_channel)
        .collect::<Result<Vec<_>, _>>()
}


