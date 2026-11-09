//! Report query module (moderation).

use uuid::Uuid;

use crate::pool::Pool;

/// Insert a report for a video or comment.
pub async fn create(
    pool: &Pool,
    reporter_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    reason: &str,
    detail: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO reports (reporter_id, target_type, target_id, reason, detail)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(reporter_id)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}
