use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{app_state::AppState, extractors::auth::AuthUser};

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
    let public_router = Router::new()
        .route("/healthz", get(system::health))
        .route("/api/system/boot-id", get(system::boot_id))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/logout", post(auth::logout));

    let protected_router = Router::new()
        .route("/api/auth/session", get(auth::session))
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
        .route_layer(middleware::from_extractor_with_state::<AuthUser, _>(
            state.clone(),
        ));

    public_router.merge(protected_router).with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use deadpool_redis::{Config as RedisConfig, Runtime as RedisRuntime};
    use sqlx::mysql::MySqlPoolOptions;
    use tower::ServiceExt;
    use waoowaoo_core::config::AppConfig;

    use super::api_router;
    use crate::app_state::AppState;

    fn test_state() -> AppState {
        let config = AppConfig {
            host: "127.0.0.1".to_string(),
            port: 3001,
            database_url: "mysql://root:pass@127.0.0.1:3306/test".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jwt_secret: "test-jwt-secret-which-is-long-enough".to_string(),
            api_encryption_key: String::new(),
            jwt_ttl_seconds: 3600,
            cors_allow_origin: vec![],
            internal_task_token: String::new(),
            billing_enabled: false,
            ark_api_key: String::new(),
            google_ai_key: String::new(),
            minimax_api_key: String::new(),
            vidu_api_key: String::new(),
            ark_api_base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            google_api_base_url: "https://generativelanguage.googleapis.com".to_string(),
            minimax_api_base_url: "https://api.minimaxi.com/v1".to_string(),
            vidu_api_base_url: "https://api.vidu.cn/ent/v2".to_string(),
            generator_http_timeout_secs: 120,
            llm_stream_chunk_timeout_ms: 180_000,
            generator_poll_interval_ms: 3_000,
            generator_poll_timeout_secs: 1_200,
            generator_retry_max_attempts: 3,
            generator_retry_backoff_ms: 1_000,
        };

        let mysql = MySqlPoolOptions::new()
            .connect_lazy(&config.database_url)
            .expect("mysql lazy pool should build for route auth tests");
        let redis = RedisConfig::from_url(&config.redis_url)
            .create_pool(Some(RedisRuntime::Tokio1))
            .expect("redis lazy pool should build for route auth tests");

        AppState::new(config, mysql, redis)
    }

    #[tokio::test]
    async fn protected_routes_reject_requests_without_token() {
        let app = api_router(test_state());

        let protected_paths = [
            "/api/auth/session",
            "/api/user/models",
            "/api/user/api-config",
            "/api/user-preference",
            "/api/projects",
            "/api/tasks",
            "/api/runs",
            "/api/asset-hub/folders",
            "/api/asset-hub/characters",
            "/api/asset-hub/picker?type=character",
        ];

        for path in protected_paths {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(path)
                        .body(Body::empty())
                        .expect("request should build"),
                )
                .await
                .expect("router should respond");

            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "expected protected route to require auth: {path}"
            );
        }
    }

    #[tokio::test]
    async fn public_routes_remain_accessible_without_token() {
        let app = api_router(test_state());

        let routes = [
            ("GET", "/healthz"),
            ("GET", "/api/system/boot-id"),
            ("POST", "/api/auth/logout"),
        ];

        for (method, path) in routes {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .expect("request should build"),
                )
                .await
                .expect("router should respond");

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "expected public route to remain open: {path}"
            );
        }
    }

    #[tokio::test]
    async fn logout_route_sets_cookie_clear_header() {
        let app = api_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);

        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("logout should emit set-cookie header");
        assert!(set_cookie.starts_with("token=;"));
        assert!(set_cookie.contains("Path=/"));
        assert!(set_cookie.contains("Max-Age=0"));
        assert!(set_cookie.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
    }
}
