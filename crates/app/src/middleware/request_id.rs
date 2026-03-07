use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::Instrument;

const X_REQUEST_ID_HEADER: &str = "x-request-id";

tokio::task_local! {
    static REQUEST_ID: String;
}

pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(|request_id| request_id.clone()).ok()
}

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(X_REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        request
            .headers_mut()
            .insert(X_REQUEST_ID_HEADER, header_value);
    }

    let span = tracing::info_span!("http.request", request_id = %request_id);
    let response_future = REQUEST_ID.scope(request_id.clone(), next.run(request));
    let mut response = response_future.instrument(span).await;
    if !response.headers().contains_key(X_REQUEST_ID_HEADER)
        && let Ok(header_value) = HeaderValue::from_str(&request_id)
    {
        response
            .headers_mut()
            .insert(X_REQUEST_ID_HEADER, header_value);
    }

    response
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn,
        routing::get,
    };
    use tower::ServiceExt;

    use super::request_id_middleware;

    #[tokio::test]
    async fn middleware_sets_request_id_when_header_absent() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .expect("x-request-id should be set");

        uuid::Uuid::parse_str(request_id).expect("x-request-id should be valid uuid");
    }

    #[tokio::test]
    async fn middleware_keeps_existing_request_id_header() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-request-id", "custom-id-123")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .expect("x-request-id should be set");

        assert_eq!(request_id, "custom-id-123");
    }
}
