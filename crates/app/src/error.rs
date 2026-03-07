use axum::{Json, http::StatusCode, response::IntoResponse};
use uuid::Uuid;
use waoowaoo_core::errors::{ApiErrorBody, AppError as CoreAppError};

use crate::middleware::request_id::current_request_id;

#[derive(Debug)]
pub struct AppError(pub CoreAppError);

impl AppError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self(CoreAppError::invalid_params(message))
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self(CoreAppError::unauthorized(message))
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self(CoreAppError::forbidden(message))
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self(CoreAppError::not_found(message))
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self(CoreAppError::conflict(message))
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self(CoreAppError::internal(message))
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let spec = self.0.code.spec();
        let status =
            StatusCode::from_u16(spec.http_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let request_id = current_request_id().unwrap_or_else(|| Uuid::new_v4().to_string());
        let body = Json(ApiErrorBody::from_app_error(&self.0, request_id.clone()));

        (status, [("x-request-id", request_id)], body).into_response()
    }
}

impl From<CoreAppError> for AppError {
    fn from(value: CoreAppError) -> Self {
        Self(value)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self(value.into())
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use uuid::Uuid;

    #[tokio::test]
    async fn into_response_sets_matching_request_id_in_header_and_body() {
        let response = AppError::invalid_params("invalid payload").into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let header_request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .expect("x-request-id header should be present")
            .to_string();

        Uuid::parse_str(&header_request_id).expect("x-request-id header should be a valid UUID");

        let (_, body) = response.into_parts();
        let body_bytes = to_bytes(body, usize::MAX)
            .await
            .expect("error response body should be readable");
        let body_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("error response body should be valid JSON");
        let body_request_id = body_json
            .get("requestId")
            .and_then(serde_json::Value::as_str)
            .expect("error response body should contain requestId");

        assert_eq!(body_request_id, header_request_id);
    }
}
