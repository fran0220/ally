use std::{collections::BTreeMap, fmt};

use serde_json::{Value, json};

use crate::errors::AppError;

use super::PromptId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptI18nErrorCode {
    PromptIdUnregistered,
    PromptTemplateNotFound,
    PromptVariableMissing,
    PromptVariableUnexpected,
    PromptPlaceholderMismatch,
}

impl PromptI18nErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PromptIdUnregistered => "PROMPT_ID_UNREGISTERED",
            Self::PromptTemplateNotFound => "PROMPT_TEMPLATE_NOT_FOUND",
            Self::PromptVariableMissing => "PROMPT_VARIABLE_MISSING",
            Self::PromptVariableUnexpected => "PROMPT_VARIABLE_UNEXPECTED",
            Self::PromptPlaceholderMismatch => "PROMPT_PLACEHOLDER_MISMATCH",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptI18nError {
    pub code: PromptI18nErrorCode,
    pub prompt_id: PromptId,
    pub message: String,
    pub details: BTreeMap<String, String>,
}

impl PromptI18nError {
    pub fn new(code: PromptI18nErrorCode, prompt_id: PromptId, message: impl Into<String>) -> Self {
        Self {
            code,
            prompt_id,
            message: message.into(),
            details: BTreeMap::new(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn to_app_error(&self) -> AppError {
        let details: Value = json!({
            "promptId": self.prompt_id.as_str(),
            "promptCode": self.code.as_str(),
            "details": self.details,
        });
        AppError::invalid_params(format!(
            "prompt-i18n {}: {}",
            self.code.as_str(),
            self.message
        ))
        .with_details(details)
    }
}

impl fmt::Display for PromptI18nError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}]: {}",
            self.code.as_str(),
            self.prompt_id.as_str(),
            self.message
        )
    }
}

impl std::error::Error for PromptI18nError {}

impl From<PromptI18nError> for AppError {
    fn from(value: PromptI18nError) -> Self {
        value.to_app_error()
    }
}
