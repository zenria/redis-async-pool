//! # Deadpool manager for asynchronous Redis connections
//!
//! redis-async-pool implements a deadpool manager for asynchronous
//! connections of the redis [crate](https://crates.io/crates/redis). Pooled connections can be used
//! as regular `redis::aio::Connection`.
//!
//! ## Features
//! - runtime agnostic (tested with tokio or async-std)
//! - optional check of connection on recycle
//! - optional ttl on connections
//!
//! ## Example
//!
//! ```rust
//! use redis::AsyncCommands;
//! use redis_async_pool::{RedisConnectionManager, RedisPool};
//!
//! // Create a pool of maximum 5, checked on reuse without ttl.
//! let pool = RedisPool::new(
//!     RedisConnectionManager::new(redis::Client::open("redis://localhost:6379")?, true, None),
//!     5,
//! );
//!
//! // get a connection with the get() asyncc method and use it as regular redis connection
//! let mut con = pool.get().await?;
//! con.set(b"key", b"value").await?;
//! let value: Vec<u8> = con.get(b"key").await?;
//! assert_eq!(value, b"value");
//! ```
//!
//! You can set a ttl for each created connection by the pool,
//! this helps avoiding huge memory consumption when keeping many connections
//! open during a too long time.

use std::{
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use deadpool::managed::RecycleError;
use redis::AsyncCommands;

pub use deadpool;

/// The redis connection pool
///
/// Use the `new` method to create a new pool. You can find
/// more information in the documentation of the `deadpool` crate.
pub type RedisPool = deadpool::managed::Pool<RedisConnection, redis::RedisError>;

/// Manages creation and destruction of redis connections.
///
pub struct RedisConnectionManager {
    client: redis::Client,
    check_on_recycle: bool,
    connection_ttl: Option<Duration>,
}

impl RedisConnectionManager {
    /// Create a new connection mananager.
    ///
    /// If `check_on_recycle` is true, before each connection reuse, an `exists` command
    /// is issued, if it fails to complete, the connection is dropped and a fresh connection.
    /// will be created.
    ///
    /// If `connection_ttl` is set, the connection will be recreated after the given duration.
    pub fn new(
        client: redis::Client,
        check_on_recycle: bool,
        connection_ttl: Option<Duration>,
    ) -> Self {
        Self {
            client,
            check_on_recycle,
            connection_ttl,
        }
    }
}

#[async_trait]
impl deadpool::managed::Manager<RedisConnection, redis::RedisError> for RedisConnectionManager {
    async fn create(&self) -> Result<RedisConnection, redis::RedisError> {
        Ok(RedisConnection {
            actual: self.client.get_async_connection().await?,
            expires_at: self
                .connection_ttl
                .as_ref()
                .map(|max_duration| Instant::now() + *max_duration),
        })
    }
    async fn recycle(
        &self,
        conn: &mut RedisConnection,
    ) -> deadpool::managed::RecycleResult<redis::RedisError> {
        if self.check_on_recycle {
            let _r: bool = conn.exists(b"key").await?;
        }
        match &conn.expires_at {
            // check if connection is expired
            Some(expires_at) => {
                if &Instant::now() >= expires_at {
                    Err(RecycleError::Message("Connection expired".to_string()))
                } else {
                    Ok(())
                }
            }
            // no expire on connections
            None => Ok(()),
        }
    }
}

/// The connection created by the pool manager.
///
/// It is Deref & DerefMut to `redis::aio::Connection` so it can be used
/// like a regular Redis asynchronous connection.
pub struct RedisConnection {
    actual: redis::aio::Connection,
    expires_at: Option<Instant>,
}

// Impl Deref & DefrefMut so the RedisConnection can be used as the real
// redis::aio::Connection

impl Deref for RedisConnection {
    type Target = redis::aio::Connection;
    fn deref(&self) -> &Self::Target {
        &self.actual
    }
}

impl DerefMut for RedisConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.actual
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
