pub mod models;

use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use sqlx::MySqlPool;

use crate::errors::AppError;

pub async fn connect_mysql(database_url: &str) -> Result<MySqlPool, AppError> {
    MySqlPool::connect(database_url)
        .await
        .map_err(|err| AppError::internal(format!("failed to connect mysql: {err}")))
}

pub fn connect_redis(redis_url: &str) -> Result<RedisPool, AppError> {
    let cfg = RedisConfig::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .map_err(|err| AppError::internal(format!("failed to create redis pool: {err}")))
}
