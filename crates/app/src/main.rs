mod app_state;
mod error;
mod extractors;
mod middleware;
mod routes;
mod watchdog;
mod worker;

use anyhow::{Result, bail};
use axum::{Router, middleware as axum_middleware};
use deadpool_redis::Pool as RedisPool;
use sqlx::MySqlPool;
use std::env;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use waoowaoo_core::{config::AppConfig, db};

use crate::{
    app_state::AppState,
    middleware::{cors::build_cors, logging::trace_layer, request_id::request_id_middleware},
    routes::api_router,
};

pub(crate) use worker::{consumer, handlers, runtime, task_context};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,tower_http=info")),
        )
        .with(fmt::layer().json())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let command = env::args().nth(1).unwrap_or_else(|| "serve".to_string());
    if !matches!(command.as_str(), "serve" | "work" | "watch") {
        bail!("unknown command: {command}. Use: serve, work, watch");
    }

    let config = AppConfig::load()?;
    let mysql = db::connect_mysql(&config.database_url).await?;
    let redis = db::connect_redis(&config.redis_url)?;

    match command.as_str() {
        "serve" => run_server(config, mysql, redis).await?,
        "work" => worker::run_worker(config, mysql, redis).await?,
        "watch" => watchdog::run_watchdog(mysql, redis).await?,
        _ => unreachable!("command is validated above"),
    }

    Ok(())
}

async fn run_server(config: AppConfig, mysql: MySqlPool, redis: RedisPool) -> Result<()> {
    let host = config.host.clone();
    let port = config.port;

    let app_state = AppState::new(config, mysql, redis);

    let app: Router = api_router(app_state.clone())
        .layer(trace_layer())
        .layer(axum_middleware::from_fn(request_id_middleware))
        .layer(build_cors(&app_state.config));

    let listener = TcpListener::bind((host.as_str(), port)).await?;
    info!(host = host, port = port, "waoowaoo rust server listening");
    axum::serve(listener, app).await?;

    Ok(())
}
