//! Domain types for the YouTube clone.
//!
//! Mirrors the DB schema and provides serde request/response types.

pub mod channel;
pub mod auth;
pub mod comment;
pub mod error;
pub mod history;
pub mod like;
pub mod subscription;
pub mod user;
pub mod video;

pub use auth::*;
pub use channel::*;
pub use comment::*;
pub use error::ApiError;
pub use history::*;
pub use like::*;
pub use subscription::*;
pub use user::*;
pub use video::*;
