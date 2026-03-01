use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    Auth,
    Content,
    Provider,
    System,
    Validation,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    InvalidParams,
    MissingConfig,
    Conflict,
    TaskNotReady,
    NoResult,
    RateLimit,
    QuotaExceeded,
    ExternalError,
    NetworkError,
    SensitiveContent,
    GenerationTimeout,
    GenerationFailed,
    WatchdogTimeout,
    WorkerExecutionError,
    InternalError,
}

#[derive(Debug, Clone, Copy)]
pub struct ErrorSpec {
    pub http_status: u16,
    pub retryable: bool,
    pub category: ErrorCategory,
    pub user_message_key: &'static str,
    pub default_message: &'static str,
}

impl ErrorCode {
    pub const fn spec(self) -> ErrorSpec {
        match self {
            ErrorCode::Unauthorized => ErrorSpec {
                http_status: 401,
                retryable: false,
                category: ErrorCategory::Auth,
                user_message_key: "errors.UNAUTHORIZED",
                default_message: "Unauthorized",
            },
            ErrorCode::Forbidden => ErrorSpec {
                http_status: 403,
                retryable: false,
                category: ErrorCategory::Auth,
                user_message_key: "errors.FORBIDDEN",
                default_message: "Forbidden",
            },
            ErrorCode::NotFound => ErrorSpec {
                http_status: 404,
                retryable: false,
                category: ErrorCategory::Validation,
                user_message_key: "errors.NOT_FOUND",
                default_message: "Resource not found",
            },
            ErrorCode::InvalidParams => ErrorSpec {
                http_status: 400,
                retryable: false,
                category: ErrorCategory::Validation,
                user_message_key: "errors.INVALID_PARAMS",
                default_message: "Invalid parameters",
            },
            ErrorCode::MissingConfig => ErrorSpec {
                http_status: 400,
                retryable: false,
                category: ErrorCategory::Validation,
                user_message_key: "errors.MISSING_CONFIG",
                default_message: "Missing required configuration",
            },
            ErrorCode::Conflict => ErrorSpec {
                http_status: 409,
                retryable: false,
                category: ErrorCategory::Validation,
                user_message_key: "errors.CONFLICT",
                default_message: "Conflict",
            },
            ErrorCode::TaskNotReady => ErrorSpec {
                http_status: 202,
                retryable: true,
                category: ErrorCategory::System,
                user_message_key: "errors.TASK_NOT_READY",
                default_message: "Task is not ready",
            },
            ErrorCode::NoResult => ErrorSpec {
                http_status: 404,
                retryable: false,
                category: ErrorCategory::System,
                user_message_key: "errors.NO_RESULT",
                default_message: "No task result",
            },
            ErrorCode::RateLimit => ErrorSpec {
                http_status: 429,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.RATE_LIMIT",
                default_message: "Rate limit exceeded",
            },
            ErrorCode::QuotaExceeded => ErrorSpec {
                http_status: 429,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.QUOTA_EXCEEDED",
                default_message: "Quota exceeded",
            },
            ErrorCode::ExternalError => ErrorSpec {
                http_status: 502,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.EXTERNAL_ERROR",
                default_message: "External service failed",
            },
            ErrorCode::NetworkError => ErrorSpec {
                http_status: 502,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.NETWORK_ERROR",
                default_message: "Network request failed",
            },
            ErrorCode::SensitiveContent => ErrorSpec {
                http_status: 422,
                retryable: false,
                category: ErrorCategory::Content,
                user_message_key: "errors.SENSITIVE_CONTENT",
                default_message: "Sensitive content detected",
            },
            ErrorCode::GenerationTimeout => ErrorSpec {
                http_status: 504,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.GENERATION_TIMEOUT",
                default_message: "Generation timed out",
            },
            ErrorCode::GenerationFailed => ErrorSpec {
                http_status: 500,
                retryable: true,
                category: ErrorCategory::Provider,
                user_message_key: "errors.GENERATION_FAILED",
                default_message: "Generation failed",
            },
            ErrorCode::WatchdogTimeout => ErrorSpec {
                http_status: 500,
                retryable: true,
                category: ErrorCategory::System,
                user_message_key: "errors.WATCHDOG_TIMEOUT",
                default_message: "Task heartbeat timeout",
            },
            ErrorCode::WorkerExecutionError => ErrorSpec {
                http_status: 500,
                retryable: true,
                category: ErrorCategory::System,
                user_message_key: "errors.WORKER_EXECUTION_ERROR",
                default_message: "Worker execution failed",
            },
            ErrorCode::InternalError => ErrorSpec {
                http_status: 500,
                retryable: false,
                category: ErrorCategory::System,
                user_message_key: "errors.INTERNAL_ERROR",
                default_message: "Internal server error",
            },
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::InvalidParams => "INVALID_PARAMS",
            ErrorCode::MissingConfig => "MISSING_CONFIG",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::TaskNotReady => "TASK_NOT_READY",
            ErrorCode::NoResult => "NO_RESULT",
            ErrorCode::RateLimit => "RATE_LIMIT",
            ErrorCode::QuotaExceeded => "QUOTA_EXCEEDED",
            ErrorCode::ExternalError => "EXTERNAL_ERROR",
            ErrorCode::NetworkError => "NETWORK_ERROR",
            ErrorCode::SensitiveContent => "SENSITIVE_CONTENT",
            ErrorCode::GenerationTimeout => "GENERATION_TIMEOUT",
            ErrorCode::GenerationFailed => "GENERATION_FAILED",
            ErrorCode::WatchdogTimeout => "WATCHDOG_TIMEOUT",
            ErrorCode::WorkerExecutionError => "WORKER_EXECUTION_ERROR",
            ErrorCode::InternalError => "INTERNAL_ERROR",
        }
    }
}
