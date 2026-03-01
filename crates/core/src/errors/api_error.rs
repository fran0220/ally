use serde::Serialize;
use uuid::Uuid;

use super::{AppError, ErrorPayload};

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub success: bool,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub error: ErrorPayload,
    pub code: String,
    pub message: String,
}

impl ApiErrorBody {
    pub fn from_app_error(value: &AppError, request_id: String) -> Self {
        let payload = value.payload();
        Self {
            success: false,
            request_id,
            code: payload.code.clone(),
            message: payload.message.clone(),
            error: payload,
        }
    }
}

impl From<&AppError> for ApiErrorBody {
    fn from(value: &AppError) -> Self {
        Self::from_app_error(value, Uuid::new_v4().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ApiErrorBody;
    use crate::errors::AppError;
    use uuid::Uuid;

    #[test]
    fn from_app_error_includes_uuid_request_id() {
        let body = ApiErrorBody::from(&AppError::invalid_params("invalid payload"));

        assert!(!body.success);
        assert_eq!(body.code, "INVALID_PARAMS");
        assert_eq!(body.message, "invalid payload");
        Uuid::parse_str(&body.request_id).expect("request_id should be a valid UUID");
    }

    #[test]
    fn serializes_request_id_as_request_id_camel_case() {
        let request_id = "00000000-0000-4000-8000-000000000000".to_string();
        let body = ApiErrorBody::from_app_error(
            &AppError::invalid_params("invalid payload"),
            request_id.clone(),
        );

        let value = serde_json::to_value(body).expect("ApiErrorBody should serialize to JSON");

        assert_eq!(
            value.get("requestId").and_then(serde_json::Value::as_str),
            Some(request_id.as_str())
        );
        assert!(value.get("request_id").is_none());
    }
}
