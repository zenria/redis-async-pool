# Deadpool manager for asynchronous Redis connections [![crates.io](https://meritbadge.herokuapp.com/redis-async-pool)](https://crates.io/crates/redis-async-pool) [![docs.rs](https://docs.rs/redis-async-pool/badge.svg)](https://docs.rs/redis-async-pool/) [![Build Status](https://travis-ci.org/zenria/redis-async-pool.svg?branch=master)](https://travis-ci.org/zenria/redis-async-pool)

 `redis-async-pool` implements a deadpool manager for asynchronous
 connections of the [redis crate](https://crates.io/crates/redis). Connections returned by the pool can be used  as regular `redis::aio::Connection`.

## Foreword

You may not need of a pool of async connections to Redis. Depending on your
workload, a multiplexed connection will be way faster. Using the [`ConnectionManager`](https://docs.rs/redis/0.17.0/redis/aio/struct.ConnectionManager.html)
provided by the redis crate, you can achieve very high performances without pooling
connections.

 ## Features

 - runtime agnostic (tested with tokio & async-std)
 - optional check of connection on recycle
 - optional ttl on connections

 ## Example

 ```rust
 use redis::AsyncCommands;
 use redis_async_pool::{RedisConnectionManager, RedisPool};

 // Create a pool of maximum 5 connections, checked on reuse without ttl.
 let pool = RedisPool::new(
     RedisConnectionManager::new(redis::Client::open("redis://localhost:6379")?, true, None),
     5,
 );

 // get a connection with the get() async method and use it as regular redis connection
 let mut con = pool.get().await?;
 con.set(b"key", b"value").await?;
 let value: Vec<u8> = con.get(b"key").await?;
 assert_eq!(value, b"value");
 ```

 You can set a ttl for each created connection by the pool,
 this helps avoiding huge memory consumption when keeping many connections
 open during a too long time.


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
