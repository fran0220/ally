mod app_state;
mod error;
mod extractors;
mod middleware;
mod routes;

use anyhow::Result;
use axum::{Router, middleware as axum_middleware};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use waoowaoo_core::{config::AppConfig, db};

use crate::{
    app_state::AppState,
    middleware::{cors::build_cors, logging::trace_layer, request_id::request_id_middleware},
    routes::api_router,
};

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

    let config = AppConfig::load()?;
    let mysql = db::connect_mysql(&config.database_url).await?;
    let redis = db::connect_redis(&config.redis_url)?;

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
