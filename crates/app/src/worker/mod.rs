pub(crate) mod consumer;
pub(crate) mod dispatcher;
pub(crate) mod handlers;
pub(crate) mod heartbeat;
pub(crate) mod runtime;
pub(crate) mod task_context;

use anyhow::Result;
use deadpool_redis::Pool as RedisPool;
use sqlx::MySqlPool;
use std::env;
use tokio::task::JoinSet;
use tracing::info;
use waoowaoo_core::config::AppConfig;

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
    billing_enabled: bool,
    mysql: &MySqlPool,
    redis: &RedisPool,
) {
    for _ in 0..concurrency {
        joins.spawn(dispatcher::run_worker_loop(
            queue,
            billing_enabled,
            mysql.clone(),
            redis.clone(),
        ));
    }
}

pub async fn run_worker(config: AppConfig, mysql: MySqlPool, redis: RedisPool) -> Result<()> {
    runtime::init(mysql.clone())?;

    let image_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_IMAGE", 20);
    let text_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_TEXT", 10);
    let video_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_VIDEO", 4);
    let voice_concurrency = read_queue_concurrency("QUEUE_CONCURRENCY_VOICE", 10);
    let billing_enabled = config.billing_enabled;

    info!(
        image_concurrency,
        text_concurrency,
        video_concurrency,
        voice_concurrency,
        billing_enabled,
        "starting worker loops"
    );

    let mut joins = JoinSet::new();
    spawn_queue_workers(
        &mut joins,
        "image",
        image_concurrency,
        billing_enabled,
        &mysql,
        &redis,
    );
    spawn_queue_workers(
        &mut joins,
        "text",
        text_concurrency,
        billing_enabled,
        &mysql,
        &redis,
    );
    spawn_queue_workers(
        &mut joins,
        "video",
        video_concurrency,
        billing_enabled,
        &mysql,
        &redis,
    );
    spawn_queue_workers(
        &mut joins,
        "voice",
        voice_concurrency,
        billing_enabled,
        &mysql,
        &redis,
    );
    joins.spawn(heartbeat::run_heartbeat(redis.clone()));

    while let Some(result) = joins.join_next().await {
        result??;
    }

    Ok(())
}
