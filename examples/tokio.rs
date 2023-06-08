use std::error::Error;
use redis::AsyncCommands;

use redis_async_pool::{RedisConnectionManager, RedisPool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = RedisPool::builder(
        RedisConnectionManager::new(redis::Client::open("redis://localhost:6379")?, true, None),
    ).max_size(5).build()?;

    let mut con = pool.get().await?;
    con.set(b"key", b"value").await?;
    let value: Vec<u8> = con.get(b"key").await?;
    assert_eq!(value, b"value");
    let exists: bool = con.exists(b"key").await?;
    println!("Key `key` exists? {}", exists);

    Ok(())
}
