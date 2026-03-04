use axum::{
    Router,
    routing::{get, post},
};

use crate::app_state::AppState;

pub mod admin;
pub mod asset_hub;
pub mod auth;
pub mod billing;
pub mod cos;
pub mod files;
pub mod media;
pub mod novel;
pub mod projects;
pub mod runs;
pub mod sse;
pub mod system;
pub mod task_submit;
pub mod tasks;
pub mod user;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(system::health))
        .route("/api/system/boot-id", get(system::boot_id))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/user/models", get(user::models))
        .route(
            "/api/user/api-config",
            get(user::get_api_config).put(user::update_api_config),
        )
        .route(
            "/api/user/api-config/test-connection",
            post(user::test_connection),
        )
        .route(
            "/api/user-preference",
            get(user::get_preference).patch(user::update_preference),
        )
        .merge(projects::router())
        .merge(tasks::router())
        .merge(runs::router())
        .merge(sse::router())
        .merge(cos::router())
        .merge(files::router())
        .merge(media::router())
        .merge(billing::router())
        .merge(asset_hub::router())
        .merge(novel::router())
        .merge(admin::router())
        .with_state(state)
}
