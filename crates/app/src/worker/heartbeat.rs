use deadpool_redis::Pool as RedisPool;
use tokio::time::{Duration, interval};
use tracing::info;

pub async fn run_heartbeat(_redis: RedisPool) -> Result<(), anyhow::Error> {
    let mut ticker = interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        info!("worker heartbeat");
    }
}
