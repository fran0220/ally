use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sqlx::MySqlPool;
use waoowaoo_core::{auth::JwtService, config::AppConfig};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub mysql: MySqlPool,
    pub redis: RedisPool,
    pub jwt: JwtService,
}

impl AppState {
    pub fn new(config: AppConfig, mysql: MySqlPool, redis: RedisPool) -> Self {
        let jwt = JwtService::from_config(&config);
        Self {
            config: Arc::new(config),
            mysql,
            redis,
            jwt,
        }
    }
}
