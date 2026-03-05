pub mod api_error;
pub mod codes;

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

pub use api_error::ApiErrorBody;
pub use codes::{ErrorCategory, ErrorCode, ErrorSpec};

#[derive(Debug, Error, Clone)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub category: ErrorCategory,
    pub user_message_key: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn payload(&self) -> ErrorPayload {
        let spec = self.code.spec();
        ErrorPayload {
            code: self.code.as_str().to_string(),
            message: self.message.clone(),
            retryable: spec.retryable,
            category: spec.category,
            user_message_key: spec.user_message_key,
            details: self.details.clone(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidParams, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    pub fn insufficient_balance(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InsufficientBalance, message)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        AppError::internal(format!("database error: {value}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::invalid_params(format!("invalid json payload: {value}"))
    }
}
