use dotenv;
use once_cell::sync::OnceCell;
use redis::{Client, RedisError};
// use redis_cluster::Cluster;
static REDIS_CLIENT: OnceCell<redis::Client> = OnceCell::new();
static REDIS_CONNECT_POOL: OnceCell<Pool> = OnceCell::new();
use deadpool_redis::{Config, Pool, Runtime};

pub async fn init_redis_pool() -> Result<(), RedisError> {
    let redis_url = dotenv::var("REDIS_URL").expect("REDIS_URL is not set");
    let client = Client::open(redis_url.clone())?;
    assert!(REDIS_CLIENT.set(client).is_ok());

    let cfg = Config::from_url(redis_url);
    if let Some(mut a) = cfg.pool {
        a.max_size = 10;
    }
    let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
    // 创建好连接池进行获取连接测试
    pool.get().await.expect("redis connect failed!!!");
    assert!(REDIS_CONNECT_POOL.set(pool).is_ok());
    Ok(())
}

pub fn get_redis_client() -> Option<&'static redis::Client> {
    REDIS_CLIENT.get()
}

pub fn get_redis_connect_pool() -> Option<&'static Pool> {
    REDIS_CONNECT_POOL.get()
}
