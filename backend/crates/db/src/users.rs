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
    /// Optional date of birth (age gating; never exposed publicly).
    pub date_of_birth: Option<chrono::NaiveDate>,
    /// When the user self-declared being 18+ (only used when no DOB is set).
    pub age_confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
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
            date_of_birth: r.date_of_birth,
            age_confirmed_at: r.age_confirmed_at,
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
        date_of_birth: row.try_get("date_of_birth")?,
        age_confirmed_at: row.try_get("age_confirmed_at")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Build a channel handle from a display name: keep only ASCII
/// alphanumerics so handles never contain spaces or punctuation
/// (YouTube-style @handles). Falls back to "user" if nothing remains.
fn sanitize_handle(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if cleaned.is_empty() {
        "user".to_string()
    } else {
        cleaned
    }
}

/// Insert a new user and their channel in one transaction.
///
/// The display name may collide with an existing one (YouTube-style —
/// names are not unique). The channel handle must stay unique, so when
/// the desired handle is taken we append a numeric suffix (name, name1,
/// name2, …) until the insert succeeds.
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

    let base = sanitize_handle(&req.channel_name);
    let mut handle = base.clone();
    let mut suffix: u32 = 0;
    loop {
        // Each attempt runs under its own savepoint: a unique-violation
        // aborts the transaction in Postgres, so without this the retry
        // would see "current transaction is aborted" instead of a fresh
        // 23505. ROLLBACK TO clears the aborted state and keeps the
        // user insert.
        sqlx::query("SAVEPOINT channel_insert")
            .execute(&mut *tx)
            .await?;
        match sqlx::query(
            r#"
            INSERT INTO channels (user_id, name, handle)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user.id)
        .bind(&req.channel_name)
        .bind(&handle)
        .execute(&mut *tx)
        .await
        {
            Ok(_) => break,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23505") => {
                sqlx::query("ROLLBACK TO SAVEPOINT channel_insert")
                    .execute(&mut *tx)
                    .await?;
                suffix += 1;
                if suffix > 100 {
                    return Err(sqlx::Error::Protocol(
                        "too many channel handle collisions".into(),
                    ));
                }
                handle = format!("{}{}", base, suffix);
            }
            Err(e) => return Err(e),
        }
    }

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
pub async fn find_by_id(pool: &Pool, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(row_to_user).transpose()
}

/// Set the user's avatar URL (channel avatar).
pub async fn update_avatar(
    pool: &Pool,
    user_id: Uuid,
    avatar_url: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("UPDATE users SET avatar_url = $2 WHERE id = $1 RETURNING *")
        .bind(user_id)
        .bind(avatar_url)
        .fetch_optional(pool)
        .await?;
    row.map(row_to_user).transpose()
}

/// Update the user's profile (name, email, bio, date of birth). All are the
/// final values the handler computed from the current row + the request;
/// `bio` is None when cleared; `date_of_birth` is Some(None) when clearing.
pub async fn update_profile(
    pool: &Pool,
    user_id: Uuid,
    name: &str,
    email: &str,
    bio: Option<String>,
    date_of_birth: Option<Option<chrono::NaiveDate>>,
) -> Result<Option<UserRow>, sqlx::Error> {
    let dob = date_of_birth.as_ref().and_then(|d| *d);
    let row = sqlx::query(
        r#"
        UPDATE users SET name = $2, email = $3, bio = $4,
            date_of_birth = CASE WHEN $5::bool THEN $6::date ELSE date_of_birth END
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(email)
    .bind(bio)
    .bind(date_of_birth.is_some())
    .bind(dob)
    .fetch_optional(pool)
    .await?;
    row.map(row_to_user).transpose()
}

/// Set (or clear with None) the user's 18+ self-confirmation timestamp.
pub async fn set_age_confirmed(
    pool: &Pool,
    user_id: Uuid,
    at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE users SET age_confirmed_at = $2 WHERE id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(at)
    .fetch_optional(pool)
    .await?;
    row.map(row_to_user).transpose()
}

/// Update the user's password hash (change-password).
pub async fn update_password_hash(
    pool: &Pool,
    user_id: Uuid,
    password_hash: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE users SET password_hash = $2 WHERE id = $1 RETURNING *",
    )
    .bind(user_id)
    .bind(password_hash)
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
