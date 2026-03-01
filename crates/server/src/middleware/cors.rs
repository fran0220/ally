use axum::http::{
    HeaderValue, Method,
    header::{self, HeaderName},
};
use tower_http::cors::{Any, CorsLayer};
use waoowaoo_core::config::AppConfig;

pub fn build_cors(config: &AppConfig) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];

    if config.cors_allow_origin.is_empty() {
        return CorsLayer::new()
            .allow_methods(methods)
            .allow_headers(Any)
            .allow_origin(Any);
    }

    let origins = config
        .cors_allow_origin
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();

    if origins.is_empty() {
        return CorsLayer::new()
            .allow_methods(methods)
            .allow_headers(Any)
            .allow_origin(Any);
    }

    // Frontend requests use credentials: include, so explicit allow-headers/origins are required.
    CorsLayer::new()
        .allow_methods(methods)
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            HeaderName::from_static("x-internal-task-token"),
            HeaderName::from_static("x-internal-user-id"),
            HeaderName::from_static("last-event-id"),
        ])
        .allow_origin(origins)
        .allow_credentials(true)
}
