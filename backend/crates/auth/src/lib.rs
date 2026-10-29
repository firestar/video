//! Auth crate: argon2 password hashing + JWT encode/decode.
//!
//! - Passwords: argon2id, m=64MB, t=3, p=4
//! - JWT: 7-day expiry, claims sub/email/name/channel/exp/iat

pub mod jwt;
pub mod password;

pub use jwt::{decode_token, encode_token};
pub use password::{hash_password, verify_password, PasswordError};

use std::time::Duration;

/// JWT token lifetime: 7 days.
pub const TOKEN_TTL: Duration = Duration::from_secs(7 * 24 * 3600);

/// Argon2id parameters per spec: m=64MB, t=3, p=4.
pub const ARGON2_MEMORY_KB: u32 = 64 * 1024;
pub const ARGON2_TIME: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 4;
