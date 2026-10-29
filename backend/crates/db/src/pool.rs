//! SQLx pool setup and migration runner.

use sqlx::postgres::PgPoolOptions;
use sqlx::{migrate, PgPool};
use std::time::Duration;

/// Re-export the pool type.
pub type Pool = PgPool;

/// Alias for the DB handle used across the app.
pub type Database = PgPool;

/// Create a pool from a connection string and run migrations.
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;
    let migrator = migrate!("./migrations");
    migrator.run(&pool).await.map_err(|e| {
        anyhow::anyhow!(
            "migration failed: {e}. \
             If a migration file changed after being applied, \
             reset the database (drop the _sqlx_migrations table) or \
             revert the migration file to match the previously applied version."
        )
    })?;
    tracing::info!(db_url_redacted = redact(database_url), "database pool ready, migrations applied");
    Ok(pool)
}

/// Redact the password in a conn string for logging.
fn redact(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let rest = &url[at + 1..];
            return format!("{scheme}{rest}");
        }
    }
    url.to_string()
}

/// Health-check query: returns the current timestamp.
pub async fn ping(pool: &PgPool) -> Result<chrono::DateTime<chrono::Utc>, sqlx::Error> {
    sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>("SELECT now()")
        .fetch_one(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password() {
        assert_eq!(
            redact("postgres://user:secret@db:5432/x"),
            "postgres://db:5432/x"
        );
        assert_eq!(redact("no-at-sign"), "no-at-sign");
    }
}
