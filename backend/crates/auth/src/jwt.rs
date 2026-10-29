//! JWT encode/decode using the `jsonwebtoken` crate.
//!
//! 7-day expiry. Claims: sub (user id), email, name, channel, exp, iat.

use domain::{AuthError, JwtClaims};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::TOKEN_TTL;

/// Encode a JWT for the given claims. `secret` is the shared HMAC key.
pub fn encode_token(secret: &str, claims: &JwtClaims) -> Result<String, AuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AuthError::Invalid(e.to_string()))?
        .as_secs() as i64;
    let mut c = claims.clone();
    c.iat = now;
    c.exp = now + TOKEN_TTL.as_secs() as i64;
    encode(
        &Header::new(Algorithm::HS256),
        &c,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Invalid(e.to_string()))
}

/// Decode and validate a JWT.
pub fn decode_token(secret: &str, token: &str) -> Result<JwtClaims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        let s = e.to_string();
        if s.contains("exp") || s.contains("expired") {
            AuthError::Expired
        } else {
            AuthError::Invalid(s)
        }
    })?;
    Ok(data.claims)
}

/// Type alias for encode/decode errors.
pub type JwtError = AuthError;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sample_claims() -> JwtClaims {
        JwtClaims {
            sub: Uuid::new_v4(),
            email: "a@b.com".into(),
            name: "A".into(),
            channel: "achannel".into(),
            exp: 0,
            iat: 0,
        }
    }

    #[test]
    fn roundtrip() {
        let secret = "test-secret-key-for-jwt";
        let claims = sample_claims();
        let token = encode_token(secret, &claims).unwrap();
        let decoded = decode_token(secret, &token).unwrap();
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, "a@b.com");
        assert_eq!(decoded.exp - decoded.iat, TOKEN_TTL.as_secs() as i64);
    }

    #[test]
    fn rejects_wrong_secret() {
        let claims = sample_claims();
        let token = encode_token("secret-a", &claims).unwrap();
        assert!(decode_token("secret-b", &token).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_token("secret", "not-a-jwt").is_err());
    }

    #[test]
    fn token_ttl_is_seven_days() {
        assert_eq!(TOKEN_TTL.as_secs(), 7 * 24 * 3600);
    }
}
