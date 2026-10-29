//! DB crate: SQLx pool setup + query modules per entity.

pub mod channels;
pub mod comments;
pub mod history;
pub mod likes;
pub mod pool;
pub mod subscriptions;
pub mod users;
pub mod videos;

pub use pool::{create_pool, Database, Pool, ping};
