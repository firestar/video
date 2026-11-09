//! DB crate: SQLx pool setup + query modules per entity.

pub mod analytics;
pub mod channels;
pub mod comments;
pub mod history;
pub mod notifications;
pub mod likes;
pub mod pool;
pub mod playlists;
pub mod subscriptions;
pub mod users;
pub mod videos;
pub mod watch_later;

pub use pool::{create_pool, Database, Pool, ping};
