mod consumer;
mod dispatcher;
mod handlers;
mod heartbeat;
mod runtime;
mod task_context;

use anyhow::Result;
use deadpool_redis::Pool as RedisPool;
use sqlx::MySqlPool;
use std::env;
use tokio::task::JoinSet;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use waoowaoo_core::{config::AppConfig, db};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .with(fmt::layer().json())
        .init();
}

fn read_queue_concurrency(env_key: &str, default: usize) -> usize {
    env::var(env_key)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn spawn_queue_workers(
    joins: &mut JoinSet<Result<()>>,
    queue: &'static str,
    concurrency: usize,
    mysql: &MySqlPool,
    redis: &RedisPool,
) {
    for _ in 0..concurrency {
        joins.spawn(dispatcher::run_worker_loop(
            queue,
            mysql.clone(),
            redis.clone(),
        ));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = AppConfig::load()?;
    let mysql = db::connect_mysql(&config.database_url).await?;
    let redis = db::connect_redis(&config.redis_url)?;
    runtime::init(mysql.clone())?;

    let image_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_IMAGE", 20);
    let text_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_TEXT", 10);
    let video_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_VIDEO", 4);
    let voice_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_VOICE", 10);

    info!(
        image_concurrency,
        text_concurrency, video_concurrency, voice_concurrency, "starting worker loops"
    );

    let mut joins = JoinSet::new();
    spawn_queue_workers(&mut joins, "image", image_concurrency, &mysql, &redis);
    spawn_queue_workers(&mut joins, "text", text_concurrency, &mysql, &redis);
    spawn_queue_workers(&mut joins, "video", video_concurrency, &mysql, &redis);
    spawn_queue_workers(&mut joins, "voice", voice_concurrency, &mysql, &redis);
    joins.spawn(heartbeat::run_heartbeat(redis.clone()));

    while let Some(result) = joins.join_next().await {
        result??;
    }

    Ok(())
}
