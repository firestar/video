//! API error type (pure, no framework deps).
//!
//! The `api` crate has its own `ApiError` with `IntoResponse`. This one is
//! a lightweight enum for shared error modeling.

use thiserror::Error;

/// Structured API error variants.
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

    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages() {
        assert_eq!(ApiError::NotFound.to_string(), "not found");
        assert!(ApiError::Unauthorized("x".into()).to_string().contains("x"));
        assert_eq!(ApiError::RateLimited.to_string(), "rate limited");
    }
}
