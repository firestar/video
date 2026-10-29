//! Channel query module.

use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::pool::Pool;
use domain::Channel;

#[derive(Debug, FromRow)]
pub struct ChannelRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub handle: String,
    pub description: Option<String>,
    pub banner_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ChannelRow> for Channel {
    fn from(r: ChannelRow) -> Self {
        Channel {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            handle: r.handle,
            description: r.description,
            banner_url: r.banner_url,
            created_at: r.created_at,
        }
    }
}

pub fn row_to_channel(row: sqlx::postgres::PgRow) -> Result<ChannelRow, sqlx::Error> {
    Ok(ChannelRow {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        name: row.try_get("name")?,
        handle: row.try_get("handle")?,
        description: row.try_get("description")?,
        banner_url: row.try_get("banner_url")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn find_by_id(pool: &Pool, id: Uuid) -> Result<Option<ChannelRow>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM channels WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_channel).transpose()?)
}

pub async fn find_by_user(pool: &Pool, user_id: Uuid) -> Result<Option<ChannelRow>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM channels WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_channel).transpose()?)
}

pub async fn update(
    pool: &Pool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    banner_url: Option<&str>,
) -> Result<Option<ChannelRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        UPDATE channels SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            banner_url = COALESCE($4, banner_url)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(banner_url)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(row_to_channel).transpose()?)
}

/// Batch-load channels by id (for attaching to video lists).
pub async fn find_by_ids(
    pool: &Pool,
    ids: &[Uuid],
) -> Result<Vec<ChannelRow>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM channels WHERE id = ANY($1)")
        .bind(&ids)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(row_to_channel).collect::<Result<Vec<_>, _>>()?)
}
