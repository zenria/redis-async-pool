//! # Deadpool manager for asynchronous Redis connections
//!
//! redis-async-pool implements a deadpool manager for asynchronous
//! connections of the [redis crate](https://crates.io/crates/redis). Connections returned by  
//! the pool can be used as regular `redis::aio::Connection`.
//!
//! ## Foreword
//!
//! You may not need of a pool of async connections to Redis. Depending on your
//! workload, a multiplexed connection will be way faster. Using the [`ConnectionManager`](https://docs.rs/redis/0.17.0/redis/aio/struct.ConnectionManager.html)
//! provided by the redis crate, you can achieve very high performances without pooling
//! connections.
//!
//! ## Features
//! - runtime agnostic (tested with tokio & async-std)
//! - optional check of connection on recycle
//! - optional ttl on connections
//!
//! ## Example
//!
//! ```rust
//! # async move {
//! use redis::AsyncCommands;
//! use redis_async_pool::{RedisConnectionManager, RedisPool};
//!
//! // Create a pool of maximum 5 connections, checked on reuse without ttl.
//!
//! let pool = RedisPool::new(
//!     RedisConnectionManager::new(redis::Client::open("redis://localhost:6379")?, true, None),
//!     5,
//! );
//!
//! // get a connection with the get() async method and use it as regular redis connection
//! let mut con = pool.get().await?;
//! con.set(b"key", b"value").await?;
//! let value: Vec<u8> = con.get(b"key").await?;
//! assert_eq!(value, b"value");
//! # }
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
use deadpool::managed::{RecycleError, RecycleResult};
use rand::Rng;
use redis::AsyncCommands;

pub use deadpool;

/// The redis connection pool
///
/// Use the `new` method to create a new pool. You can find
/// more information in the documentation of the `deadpool` crate.
pub type RedisPool = deadpool::managed::Pool<RedisConnection, redis::RedisError>;

/// Time to live of a connection
pub enum Ttl {
    /// Connection will expire after the given duration
    Simple(Duration),
    /// Connection will expire after at least `min` time and at most
    /// `min + fuzz` time.
    ///
    /// Actual ttl is computed at connection creation by adding `min` duration to
    /// a random duration between 0 and `fuzz`.
    Fuzzy { min: Duration, fuzz: Duration },
    /// The connection will not been reused. A new connection will be created
    /// for each `get()` on the pool.
    ///
    /// Enabling Once ttl means the pool will not keep any connection opened.
    /// So it won't really act as a pool of connection.
    Once,
}

/// Manages creation and destruction of redis connections.
///
pub struct RedisConnectionManager {
    client: redis::Client,
    check_on_recycle: bool,
    connection_ttl: Option<Ttl>,
}

impl RedisConnectionManager {
    /// Create a new connection manager.
    ///
    /// If `check_on_recycle` is true, before each connection reuse, an `exists` command
    /// is issued, if it fails to complete, the connection is dropped and a fresh connection
    /// is created.
    ///
    /// If `connection_ttl` is set, the connection will be recreated after the given duration.
    pub fn new(client: redis::Client, check_on_recycle: bool, connection_ttl: Option<Ttl>) -> Self {
        Self {
            client,
            check_on_recycle,
            connection_ttl,
        }
    }
}

#[async_trait]
impl deadpool::managed::Manager for RedisConnectionManager {
    type Error = redis::RedisError;
    type Type = RedisConnection;

    async fn create(&self) -> Result<RedisConnection, redis::RedisError> {
        Ok(RedisConnection {
            actual: self.client.get_async_connection().await?,
            expires_at: self
                .connection_ttl
                .as_ref()
                .map(|max_duration| match max_duration {
                    Ttl::Simple(ttl) => Instant::now() + *ttl,
                    Ttl::Fuzzy { min, fuzz } => {
                        Instant::now()
                            + *min
                            + Duration::from_secs_f64(
                                rand::thread_rng().gen_range((0.0)..fuzz.as_secs_f64()),
                            )
                    }
                    // already expired ;)
                    Ttl::Once => Instant::now(),
                }),
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

// Impl Deref & DerefMut so the RedisConnection can be used as the real
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

impl AsMut<redis::aio::Connection> for RedisConnection {
    fn as_mut(&mut self) -> &mut redis::aio::Connection {
        &mut self.actual
    }
}

impl AsRef<redis::aio::Connection> for RedisConnection {
    fn as_ref(&self) -> &redis::aio::Connection {
        &self.actual
    }
}
