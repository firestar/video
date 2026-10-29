//! Auth request/response and JWT claim types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims carried in an auth token.
///
/// 7-day expiry enforced at encode time; `exp`/`iat` are unix seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject — the user id.
    pub sub: Uuid,
    pub email: String,
    pub name: String,
    /// Channel handle for this user.
    pub channel: String,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// Issued at (unix seconds).
    pub iat: i64,
}

/// Error from token validation.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("token invalid: {0}")]
    Invalid(String),
    #[error("token expired")]
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_serialize() {
        let c = JwtClaims {
            sub: Uuid::new_v4(),
            email: "a@b.com".into(),
            name: "A".into(),
            channel: "achannel".into(),
            exp: 123,
            iat: 100,
        };
        let j = serde_json::to_string(&c).unwrap();
        assert!(j.contains("\"sub\""));
        assert!(j.contains("\"email\""));
    }
}
