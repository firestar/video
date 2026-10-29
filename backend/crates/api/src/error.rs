//! API error type with IntoResponse.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden")]
    Forbidden,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("rate limited")]
    RateLimited,

    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl From<domain::AuthError> for ApiError {
    fn from(e: domain::AuthError) -> Self {
        match e {
            domain::AuthError::Expired => ApiError::Unauthorized("token expired".into()),
            domain::AuthError::Invalid(msg) => ApiError::Unauthorized(msg),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            ApiError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (
            status,
            Json(json!({ "error": { "message": message } })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => ApiError::NotFound,
            sqlx::Error::Database(db)
                if db.code().as_deref() == Some("23505") =>
            {
                ApiError::Conflict("already exists".into())
            }
            _ => ApiError::Internal(anyhow::anyhow!("db: {e}")),
        }
    }
}

impl From<auth::PasswordError> for ApiError {
    fn from(e: auth::PasswordError) -> Self {
        ApiError::Internal(anyhow::anyhow!("password: {e}"))
    }
}
