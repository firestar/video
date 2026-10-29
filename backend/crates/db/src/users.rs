//! User query module.

use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::pool::Pool;
use domain::{RegisterRequest, User};

/// DB row for a user.
#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub channel_name: String,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            email: r.email,
            name: r.name,
            channel_name: r.channel_name,
            avatar_url: r.avatar_url,
            bio: r.bio,
            created_at: r.created_at,
        }
    }
}

fn row_to_user(row: sqlx::postgres::PgRow) -> Result<UserRow, sqlx::Error> {
    Ok(UserRow {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        channel_name: row.try_get("channel_name")?,
        password_hash: row.try_get("password_hash")?,
        avatar_url: row.try_get("avatar_url")?,
        bio: row.try_get("bio")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Insert a new user and their channel in one transaction.
pub async fn create(
    pool: &Pool,
    req: &RegisterRequest,
    password_hash: &str,
) -> Result<UserRow, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let user = row_to_user(
        sqlx::query(
            r#"
            INSERT INTO users (email, name, channel_name, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&req.email)
        .bind(&req.name)
        .bind(&req.channel_name)
        .bind(password_hash)
        .fetch_one(&mut *tx)
        .await?,
    )?;

    sqlx::query(
        r#"
        INSERT INTO channels (user_id, name, handle)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user.id)
    .bind(&req.channel_name)
    .bind(&req.channel_name)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(user)
}

/// Look up a user by email (for login).
pub async fn find_by_email(pool: &Pool, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    row.map(row_to_user).transpose()
}

/// Look up a user by id.
/// Look up a user by id.
pub async fn find_by_id(pool: &Pool, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(row_to_user).transpose()
}

/// Batch-load users by id (for attaching author info to comments).
pub async fn find_by_ids(
    pool: &Pool,
    ids: &[Uuid],
) -> Result<Vec<UserRow>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM users WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(pool)
        .await?;
    rows.into_iter().map(row_to_user).collect::<Result<Vec<_>, _>>()
}
