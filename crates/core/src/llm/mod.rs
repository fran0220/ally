use std::{
    cmp,
    future::Future,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::MySqlPool;
use tokio::time::timeout;

use crate::{
    api_config::UnifiedModelType,
    errors::{AppError, ErrorCode},
    media,
    runtime::resolve_model_with_provider,
};

const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 120;
const DEFAULT_STREAM_CHUNK_TIMEOUT_MS: u64 = 180_000;
const MAX_SSE_BUFFER_BYTES: usize = 4 * 1024 * 1024;
const MAX_ERROR_BODY_CHARS: usize = 1_024;
const ANTHROPIC_MAX_TOKENS: u64 = 8_192;
const ANTHROPIC_THINKING_BUDGET_TOKENS: u64 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatStreamChunkKind {
    Text,
    Reasoning,
}

impl ChatStreamChunkKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Reasoning => "reasoning",
        }
    }

    pub const fn lane(self) -> &'static str {
        match self {
            Self::Text => "main",
            Self::Reasoning => "reasoning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatStreamChunk {
    pub kind: ChatStreamChunkKind,
    pub delta: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatStreamUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct ChatStreamOptions {
    pub temperature: Option<f32>,
    pub reasoning: bool,
    pub reasoning_effort: Option<String>,
    pub chunk_timeout: Option<Duration>,
}

impl Default for ChatStreamOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            reasoning: true,
            reasoning_effort: Some("high".to_string()),
            chunk_timeout: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatStreamResult {
    pub text: String,
    pub reasoning: String,
    pub usage: ChatStreamUsage,
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS))
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn normalize_reasoning_effort(raw: Option<&str>) -> &'static str {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return "high";
    };

    match raw.to_ascii_lowercase().as_str() {
        "minimal" => "minimal",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        _ => "high",
    }
}

fn resolve_stream_chunk_timeout(options: &ChatStreamOptions) -> Duration {
    if let Some(timeout) = options.chunk_timeout {
        return cmp::max(timeout, Duration::from_secs(1));
    }

    std::env::var("LLM_STREAM_CHUNK_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_STREAM_CHUNK_TIMEOUT_MS))
}

fn to_gemini_thinking_level(effort: &str) -> &'static str {
    match effort {
        "minimal" => "low",
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        _ => "high",
    }
}

fn value_to_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|item| match item {
        Value::Number(number) => number.as_u64().or_else(|| {
            number
                .as_i64()
                .and_then(|signed| u64::try_from(signed).ok())
        }),
        Value::String(raw) => raw.trim().parse::<u64>().ok(),
        _ => None,
    })
}

fn truncate_error_body(raw: &str) -> String {
    let mut trimmed = String::with_capacity(raw.len().min(MAX_ERROR_BODY_CHARS));
    for (index, ch) in raw.chars().enumerate() {
        if index >= MAX_ERROR_BODY_CHARS {
            trimmed.push_str(" ...(truncated)");
            return trimmed;
        }
        trimmed.push(ch);
    }
    trimmed
}

fn is_sensitive_content_text(raw: &str) -> bool {
    let normalized = raw.to_ascii_lowercase();
    if normalized.contains("sensitive_content")
        || normalized.contains("prohibited_content")
        || normalized.contains("request_body_blocked")
        || normalized.contains("content_filter")
    {
        return true;
    }

    normalized.contains("safety")
        && (normalized.contains("block")
            || normalized.contains("prohibit")
            || normalized.contains("filter"))
}

fn is_sensitive_content_json(value: &Value) -> bool {
    match value {
        Value::String(raw) => is_sensitive_content_text(raw),
        Value::Array(items) => items.iter().any(is_sensitive_content_json),
        Value::Object(map) => {
            if let Some(finish_reason) = map
                .get("finishReason")
                .or_else(|| map.get("finish_reason"))
                .and_then(Value::as_str)
            {
                let normalized = finish_reason.to_ascii_uppercase();
                if normalized == "SAFETY"
                    || normalized == "PROHIBITED_CONTENT"
                    || normalized == "CONTENT_FILTER"
                    || normalized == "REQUEST_BODY_BLOCKED"
                {
                    return true;
                }
            }

            map.values().any(is_sensitive_content_json)
        }
        _ => false,
    }
}

fn normalize_provider_http_error(provider: &str, status: StatusCode, body: &str) -> AppError {
    let body = truncate_error_body(body);
    if is_sensitive_content_text(&body) {
        return AppError::new(
            ErrorCode::SensitiveContent,
            "SENSITIVE_CONTENT: content rejected by provider safety policy",
        )
        .with_details(json!({
            "provider": provider,
            "status": status.as_u16(),
            "body": body,
        }));
    }

    let (code, message) = if status == StatusCode::TOO_MANY_REQUESTS {
        (
            ErrorCode::RateLimit,
            format!("{provider} request rate limited ({status}): {body}"),
        )
    } else if status == StatusCode::BAD_REQUEST {
        (
            ErrorCode::InvalidParams,
            format!("{provider} request invalid ({status}): {body}"),
        )
    } else if status == StatusCode::REQUEST_TIMEOUT || status == StatusCode::GATEWAY_TIMEOUT {
        (
            ErrorCode::GenerationTimeout,
            format!("task stream timeout: provider returned status {status}"),
        )
    } else {
        (
            ErrorCode::ExternalError,
            format!("{provider} request failed ({status}): {body}"),
        )
    };

    AppError::new(code, message).with_details(json!({
        "provider": provider,
        "status": status.as_u16(),
        "body": body,
    }))
}

fn extract_error_message(value: &Value) -> String {
    match value {
        Value::String(message) => message.to_string(),
        Value::Object(map) => {
            if let Some(message) = map.get("message").and_then(Value::as_str) {
                return message.to_string();
            }
            if let Some(error) = map.get("error") {
                let nested = extract_error_message(error);
                if !nested.trim().is_empty() {
                    return nested;
                }
            }
            if let Some(detail) = map.get("detail") {
                let nested = extract_error_message(detail);
                if !nested.trim().is_empty() {
                    return nested;
                }
            }
            String::new()
        }
        Value::Array(items) => items
            .iter()
            .map(extract_error_message)
            .find(|message| !message.trim().is_empty())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn normalize_provider_stream_error(provider: &str, value: &Value) -> AppError {
    let message = extract_error_message(value);
    if is_sensitive_content_json(value) || is_sensitive_content_text(&message) {
        return AppError::new(
            ErrorCode::SensitiveContent,
            "SENSITIVE_CONTENT: content rejected by provider safety policy",
        )
        .with_details(json!({
            "provider": provider,
            "error": value,
        }));
    }

    AppError::new(
        ErrorCode::ExternalError,
        if message.trim().is_empty() {
            format!("{provider} stream returned an error")
        } else {
            format!("{provider} stream error: {message}")
        },
    )
    .with_details(json!({
        "provider": provider,
        "error": value,
    }))
}

fn normalize_network_error(context: &str, error: reqwest::Error) -> AppError {
    AppError::new(ErrorCode::NetworkError, format!("{context}: {error}"))
}

fn stream_timeout_error(chunk_timeout: Duration) -> AppError {
    AppError::new(
        ErrorCode::GenerationTimeout,
        format!(
            "task stream timeout: no chunk received within {}s",
            chunk_timeout.as_secs()
        ),
    )
    .with_details(json!({
        "timeoutMs": chunk_timeout.as_millis(),
    }))
}

fn is_reasoning_unsupported_message(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    let mentions_reasoning = normalized.contains("reasoning")
        || normalized.contains("thinking")
        || normalized.contains("reasoning_effort")
        || normalized.contains("thinkinglevel")
        || normalized.contains("thinkingconfig");
    let indicates_unsupported = normalized.contains("unsupported")
        || normalized.contains("not support")
        || normalized.contains("unknown")
        || normalized.contains("unrecognized")
        || normalized.contains("invalid")
        || normalized.contains("unexpected")
        || normalized.contains("not allowed")
        || normalized.contains("does not have field");

    mentions_reasoning && indicates_unsupported
}

fn should_retry_without_reasoning(error: &AppError) -> bool {
    is_reasoning_unsupported_message(&error.message)
}

fn normalize_sse_chunk(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn extract_sse_data(raw_event: &str) -> Option<String> {
    let mut lines = Vec::new();
    for line in raw_event.lines() {
        if let Some(data) = line.strip_prefix("data:") {
            lines.push(data.trim_start().to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn drain_sse_payloads(buffer: &mut String) -> Vec<String> {
    let mut payloads = Vec::new();

    while let Some(index) = buffer.find("\n\n") {
        let raw_event = buffer[..index].to_string();
        buffer.drain(..index + 2);

        if let Some(payload) = extract_sse_data(&raw_event) {
            payloads.push(payload);
        }
    }

    payloads
}

async fn read_next_sse_payload_batch(
    response: &mut reqwest::Response,
    chunk_timeout: Duration,
    buffer: &mut String,
) -> Result<Option<Vec<String>>, AppError> {
    loop {
        let chunk = timeout(chunk_timeout, response.chunk())
            .await
            .map_err(|_| stream_timeout_error(chunk_timeout))?
            .map_err(|error| normalize_network_error("llm stream chunk read failed", error))?;

        let Some(chunk) = chunk else {
            let trailing = buffer.trim().to_string();
            buffer.clear();
            if trailing.is_empty() {
                return Ok(None);
            }

            let payload = extract_sse_data(&trailing);
            return Ok(payload.map(|item| vec![item]));
        };

        buffer.push_str(&normalize_sse_chunk(&chunk));
        if buffer.len() > MAX_SSE_BUFFER_BYTES {
            return Err(AppError::new(
                ErrorCode::ExternalError,
                "llm stream payload too large",
            ));
        }

        let payloads = drain_sse_payloads(buffer);
        if !payloads.is_empty() {
            return Ok(Some(payloads));
        }
    }
}

fn collect_text_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .map(collect_text_value)
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.to_string();
            }
            if let Some(content) = map.get("content") {
                let value = collect_text_value(content);
                if !value.is_empty() {
                    return value;
                }
            }
            if let Some(delta) = map.get("delta") {
                let value = collect_text_value(delta);
                if !value.is_empty() {
                    return value;
                }
            }
            if let Some(output_text) = map.get("output_text") {
                let value = collect_text_value(output_text);
                if !value.is_empty() {
                    return value;
                }
            }
            if let Some(message) = map.get("message") {
                let value = collect_text_value(message);
                if !value.is_empty() {
                    return value;
                }
            }
            if let Some(parts) = map.get("parts") {
                let value = collect_text_value(parts);
                if !value.is_empty() {
                    return value;
                }
            }

            String::new()
        }
        _ => String::new(),
    }
}

fn extract_completion_parts_from_content(content: &Value) -> (String, String) {
    if let Some(text) = content.as_str() {
        return (text.to_string(), String::new());
    }

    let Some(parts) = content.as_array() else {
        return (collect_text_value(content), String::new());
    };

    let mut text = String::new();
    let mut reasoning = String::new();
    for part in parts {
        if let Some(value) = part.as_str() {
            text.push_str(value);
            continue;
        }

        let Some(obj) = part.as_object() else {
            continue;
        };

        let kind = obj
            .get("type")
            .and_then(Value::as_str)
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        let value = obj
            .get("text")
            .or_else(|| obj.get("content"))
            .or_else(|| obj.get("delta"))
            .map(collect_text_value)
            .unwrap_or_default();
        if value.is_empty() {
            continue;
        }

        if kind.contains("reason") || kind.contains("think") {
            reasoning.push_str(&value);
        } else {
            text.push_str(&value);
        }
    }

    (text, reasoning)
}

fn extract_openai_delta_parts(chunk: &Value) -> (String, String) {
    let delta = chunk
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("delta"))
        .cloned()
        .unwrap_or(Value::Null);

    let (content_text, content_reasoning) =
        extract_completion_parts_from_content(delta.get("content").unwrap_or(&Value::Null));
    let response_text = chunk
        .get("response")
        .and_then(|item| item.get("output_text"))
        .and_then(|item| item.get("delta"))
        .map(collect_text_value)
        .unwrap_or_default();
    let response_reasoning = chunk
        .get("response")
        .and_then(|item| item.get("reasoning"))
        .and_then(|item| item.get("delta"))
        .map(collect_text_value)
        .unwrap_or_default();

    let text_delta = if !content_text.is_empty() {
        content_text
    } else {
        delta
            .get("output_text")
            .or_else(|| delta.get("text"))
            .map(collect_text_value)
            .filter(|value| !value.is_empty())
            .unwrap_or(response_text)
    };

    let explicit_reasoning = [
        "reasoning",
        "reasoning_content",
        "reasoningContent",
        "thinking",
        "reasoning_details",
    ]
    .iter()
    .filter_map(|key| delta.get(*key))
    .map(collect_text_value)
    .find(|value| !value.is_empty())
    .unwrap_or_default();

    let reasoning_delta = if !content_reasoning.is_empty() {
        content_reasoning
    } else if !explicit_reasoning.is_empty() {
        explicit_reasoning
    } else {
        response_reasoning
    };

    (text_delta, reasoning_delta)
}

fn extract_openai_usage(chunk: &Value) -> Option<ChatStreamUsage> {
    let usage = chunk.get("usage")?;
    let prompt_tokens = value_to_u64(
        usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .or_else(|| usage.get("promptTokenCount")),
    );
    let completion_tokens = value_to_u64(
        usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .or_else(|| usage.get("completionTokenCount")),
    );

    if prompt_tokens.is_none() && completion_tokens.is_none() {
        return None;
    }

    Some(ChatStreamUsage {
        prompt_tokens: prompt_tokens.unwrap_or(0),
        completion_tokens: completion_tokens.unwrap_or(0),
    })
}

fn is_gemini_reasoning_part(part: &Value) -> bool {
    let Some(part) = part.as_object() else {
        return false;
    };

    if part.get("thought").and_then(Value::as_bool) == Some(true) {
        return true;
    }

    part.get("type")
        .and_then(Value::as_str)
        .map(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized.contains("thought") || normalized.contains("reason")
        })
        .unwrap_or(false)
}

fn normalize_gemini_chunk(chunk: &Value) -> Value {
    if let Some(items) = chunk.as_array()
        && let Some(first) = items.first()
    {
        return first.clone();
    }
    chunk.clone()
}

fn extract_gemini_stream_parts(chunk: &Value) -> (String, String) {
    let chunk = normalize_gemini_chunk(chunk);
    let parts = chunk
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("content"))
        .and_then(|item| item.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut text = String::new();
    let mut reasoning = String::new();
    for part in parts {
        let value = part.get("text").map(collect_text_value).unwrap_or_default();
        if value.is_empty() {
            continue;
        }

        if is_gemini_reasoning_part(&part) {
            reasoning.push_str(&value);
        } else {
            text.push_str(&value);
        }
    }

    (text, reasoning)
}

fn extract_gemini_usage(chunk: &Value) -> Option<ChatStreamUsage> {
    let chunk = normalize_gemini_chunk(chunk);
    let usage = chunk
        .get("usageMetadata")
        .or_else(|| chunk.get("usage"))
        .or_else(|| {
            chunk
                .get("response")
                .and_then(|item| item.get("usageMetadata"))
        })?;

    let prompt_tokens = value_to_u64(
        usage
            .get("promptTokenCount")
            .or_else(|| usage.get("prompt_tokens"))
            .or_else(|| usage.get("input_tokens")),
    );
    let completion_tokens = value_to_u64(
        usage
            .get("candidatesTokenCount")
            .or_else(|| usage.get("completion_tokens"))
            .or_else(|| usage.get("output_tokens")),
    )
    .or_else(|| {
        let total_tokens = value_to_u64(
            usage
                .get("totalTokenCount")
                .or_else(|| usage.get("total_tokens")),
        )?;
        Some(total_tokens.saturating_sub(prompt_tokens.unwrap_or(0)))
    });

    if prompt_tokens.is_none() && completion_tokens.is_none() {
        return None;
    }

    Some(ChatStreamUsage {
        prompt_tokens: prompt_tokens.unwrap_or(0),
        completion_tokens: completion_tokens.unwrap_or(0),
    })
}

fn extract_anthropic_stream_parts(chunk: &Value) -> (String, String) {
    let event_type = chunk
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let source = match event_type {
        "content_block_start" => chunk.get("content_block").unwrap_or(&Value::Null),
        "content_block_delta" => chunk.get("delta").unwrap_or(&Value::Null),
        _ => return (String::new(), String::new()),
    };

    let source_type = source
        .get("type")
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    let text_value = source
        .get("text")
        .or_else(|| source.get("delta"))
        .map(collect_text_value)
        .unwrap_or_default();
    let thinking_value = source
        .get("thinking")
        .or_else(|| source.get("reasoning"))
        .map(collect_text_value)
        .unwrap_or_default();

    if source_type.contains("think") || source_type.contains("reason") {
        return if !thinking_value.is_empty() {
            (String::new(), thinking_value)
        } else {
            (String::new(), text_value)
        };
    }

    if source_type.contains("text") {
        return (text_value, String::new());
    }

    if !thinking_value.is_empty() {
        (String::new(), thinking_value)
    } else {
        (text_value, String::new())
    }
}

fn extract_anthropic_usage_tokens(chunk: &Value) -> (Option<u64>, Option<u64>) {
    let prompt_tokens = value_to_u64(
        chunk
            .pointer("/message/usage/input_tokens")
            .or_else(|| chunk.pointer("/usage/input_tokens"))
            .or_else(|| chunk.pointer("/message/usage/prompt_tokens"))
            .or_else(|| chunk.pointer("/usage/prompt_tokens")),
    );
    let completion_tokens = value_to_u64(
        chunk
            .pointer("/usage/output_tokens")
            .or_else(|| chunk.pointer("/message/usage/output_tokens"))
            .or_else(|| chunk.pointer("/usage/completion_tokens"))
            .or_else(|| chunk.pointer("/message/usage/completion_tokens")),
    );

    (prompt_tokens, completion_tokens)
}

fn merge_stream_piece(target: &mut String, incoming: &str) -> String {
    if incoming.is_empty() {
        return String::new();
    }

    if target.is_empty() {
        target.push_str(incoming);
        return incoming.to_string();
    }

    if incoming.starts_with(target.as_str()) {
        let delta = incoming[target.len()..].to_string();
        target.push_str(&delta);
        return delta;
    }

    if target.ends_with(incoming) {
        return String::new();
    }

    let mut overlap = 0usize;
    for (index, _) in incoming.char_indices().skip(1) {
        if target.ends_with(&incoming[..index]) {
            overlap = index;
        }
    }

    let delta = incoming[overlap..].to_string();
    if !delta.is_empty() {
        target.push_str(&delta);
    }
    delta
}

async fn emit_stream_piece<F, Fut>(
    target: &mut String,
    piece: String,
    kind: ChatStreamChunkKind,
    on_chunk: &mut F,
) -> Result<(), AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let delta = merge_stream_piece(target, &piece);
    if delta.is_empty() {
        return Ok(());
    }

    on_chunk(ChatStreamChunk { kind, delta }).await
}

fn openai_chat_completions_endpoint(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

fn gemini_generate_content_endpoint(base_url: &str, model_id: &str, stream: bool) -> String {
    let suffix = if stream {
        ":streamGenerateContent?alt=sse"
    } else {
        ":generateContent"
    };

    if base_url.contains("/v1beta") {
        format!(
            "{}/models/{}{}",
            base_url.trim_end_matches('/'),
            model_id,
            suffix
        )
    } else {
        format!(
            "{}/v1beta/models/{}{}",
            base_url.trim_end_matches('/'),
            model_id,
            suffix
        )
    }
}

fn anthropic_messages_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1/messages") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/messages")
    } else {
        format!("{trimmed}/v1/messages")
    }
}

async fn stream_with_openai_compatible<F, Fut>(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    options: &ChatStreamOptions,
    reasoning_enabled: bool,
    on_chunk: &mut F,
) -> Result<ChatStreamResult, AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let mut payload = Map::new();
    payload.insert("model".to_string(), Value::String(model_id.to_string()));
    payload.insert("messages".to_string(), serde_json::to_value(messages)?);
    payload.insert(
        "temperature".to_string(),
        json!(options.temperature.unwrap_or(0.7)),
    );
    payload.insert("stream".to_string(), Value::Bool(true));
    payload.insert(
        "stream_options".to_string(),
        json!({ "include_usage": true }),
    );

    if reasoning_enabled {
        let effort = normalize_reasoning_effort(options.reasoning_effort.as_deref());
        payload.insert("reasoning".to_string(), json!({ "effort": effort }));
        payload.insert(
            "reasoning_effort".to_string(),
            Value::String(effort.to_string()),
        );
    }

    let mut response = http_client()
        .post(openai_chat_completions_endpoint(base_url))
        .bearer_auth(api_key)
        .json(&Value::Object(payload))
        .send()
        .await
        .map_err(|error| normalize_network_error("openai-compatible request failed", error))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(normalize_provider_http_error(
            "openai-compatible",
            status,
            &body,
        ));
    }

    let mut text = String::new();
    let mut reasoning = String::new();
    let mut usage = ChatStreamUsage::default();
    let chunk_timeout = resolve_stream_chunk_timeout(options);
    let mut sse_buffer = String::new();
    let mut done = false;
    while !done {
        let Some(payloads) =
            read_next_sse_payload_batch(&mut response, chunk_timeout, &mut sse_buffer).await?
        else {
            break;
        };

        for payload in payloads {
            if payload.trim() == "[DONE]" {
                done = true;
                break;
            }

            let value: Value = serde_json::from_str(&payload).map_err(|error| {
                AppError::new(
                    ErrorCode::ExternalError,
                    format!("openai-compatible stream payload is invalid json: {error}"),
                )
            })?;

            if let Some(error) = value.get("error") {
                return Err(normalize_provider_stream_error("openai-compatible", error));
            }
            if is_sensitive_content_json(&value) {
                return Err(AppError::new(
                    ErrorCode::SensitiveContent,
                    "SENSITIVE_CONTENT: content rejected by provider safety policy",
                ));
            }

            if let Some(stream_usage) = extract_openai_usage(&value) {
                usage = stream_usage;
            }

            let (text_piece, reasoning_piece) = extract_openai_delta_parts(&value);
            emit_stream_piece(
                &mut reasoning,
                reasoning_piece,
                ChatStreamChunkKind::Reasoning,
                on_chunk,
            )
            .await?;
            emit_stream_piece(&mut text, text_piece, ChatStreamChunkKind::Text, on_chunk).await?;
        }
    }

    Ok(ChatStreamResult {
        text,
        reasoning,
        usage,
    })
}

async fn stream_with_gemini_compatible<F, Fut>(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    options: &ChatStreamOptions,
    reasoning_enabled: bool,
    on_chunk: &mut F,
) -> Result<ChatStreamResult, AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let system_prompt = messages
        .iter()
        .filter(|item| item.role == "system")
        .map(|item| item.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let contents = messages
        .iter()
        .filter(|item| item.role != "system")
        .map(|item| {
            json!({
                "role": if item.role == "assistant" { "model" } else { "user" },
                "parts": [{ "text": item.content }],
            })
        })
        .collect::<Vec<_>>();

    let effort = normalize_reasoning_effort(options.reasoning_effort.as_deref());
    let mut config = Map::new();
    config.insert(
        "temperature".to_string(),
        json!(options.temperature.unwrap_or(0.7)),
    );
    if !system_prompt.trim().is_empty() {
        config.insert(
            "systemInstruction".to_string(),
            json!({ "parts": [{ "text": system_prompt }] }),
        );
    }
    if reasoning_enabled {
        config.insert(
            "thinkingConfig".to_string(),
            json!({
                "thinkingLevel": to_gemini_thinking_level(effort),
                "includeThoughts": true,
            }),
        );
    }

    let payload = json!({
        "contents": contents,
        "config": config,
    });

    let mut response = http_client()
        .post(gemini_generate_content_endpoint(base_url, model_id, true))
        .header("x-goog-api-key", api_key)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|error| normalize_network_error("gemini-compatible request failed", error))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(normalize_provider_http_error(
            "gemini-compatible",
            status,
            &body,
        ));
    }

    let mut text = String::new();
    let mut reasoning = String::new();
    let mut usage = ChatStreamUsage::default();
    let chunk_timeout = resolve_stream_chunk_timeout(options);
    let mut sse_buffer = String::new();
    let mut done = false;
    while !done {
        let Some(payloads) =
            read_next_sse_payload_batch(&mut response, chunk_timeout, &mut sse_buffer).await?
        else {
            break;
        };

        for payload in payloads {
            if payload.trim() == "[DONE]" {
                done = true;
                break;
            }

            let value: Value = serde_json::from_str(&payload).map_err(|error| {
                AppError::new(
                    ErrorCode::ExternalError,
                    format!("gemini-compatible stream payload is invalid json: {error}"),
                )
            })?;

            if let Some(error) = value.get("error") {
                return Err(normalize_provider_stream_error("gemini-compatible", error));
            }
            if is_sensitive_content_json(&value) {
                return Err(AppError::new(
                    ErrorCode::SensitiveContent,
                    "SENSITIVE_CONTENT: content rejected by provider safety policy",
                ));
            }

            if let Some(stream_usage) = extract_gemini_usage(&value) {
                usage = stream_usage;
            }

            let (text_piece, reasoning_piece) = extract_gemini_stream_parts(&value);
            emit_stream_piece(
                &mut reasoning,
                reasoning_piece,
                ChatStreamChunkKind::Reasoning,
                on_chunk,
            )
            .await?;
            emit_stream_piece(&mut text, text_piece, ChatStreamChunkKind::Text, on_chunk).await?;
        }
    }

    Ok(ChatStreamResult {
        text,
        reasoning,
        usage,
    })
}

async fn stream_with_anthropic<F, Fut>(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    options: &ChatStreamOptions,
    reasoning_enabled: bool,
    on_chunk: &mut F,
) -> Result<ChatStreamResult, AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let system_prompt = messages
        .iter()
        .filter(|item| item.role == "system")
        .map(|item| item.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let anthropic_messages = messages
        .iter()
        .filter(|item| item.role != "system")
        .map(|item| {
            json!({
                "role": if item.role == "assistant" { "assistant" } else { "user" },
                "content": item.content,
            })
        })
        .collect::<Vec<_>>();

    if anthropic_messages.is_empty() {
        return Err(AppError::invalid_params(
            "anthropic stream requires at least one non-system message",
        ));
    }

    let mut payload = Map::new();
    payload.insert("model".to_string(), Value::String(model_id.to_string()));
    payload.insert(
        "max_tokens".to_string(),
        Value::Number(ANTHROPIC_MAX_TOKENS.into()),
    );
    payload.insert(
        "temperature".to_string(),
        json!(options.temperature.unwrap_or(0.7)),
    );
    payload.insert("stream".to_string(), Value::Bool(true));
    payload.insert("messages".to_string(), Value::Array(anthropic_messages));
    if !system_prompt.trim().is_empty() {
        payload.insert("system".to_string(), Value::String(system_prompt));
    }
    if reasoning_enabled {
        payload.insert(
            "thinking".to_string(),
            json!({
                "type": "enabled",
                "budget_tokens": ANTHROPIC_THINKING_BUDGET_TOKENS,
            }),
        );
    }

    let mut response = http_client()
        .post(anthropic_messages_endpoint(base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&Value::Object(payload))
        .send()
        .await
        .map_err(|error| normalize_network_error("anthropic request failed", error))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(normalize_provider_http_error("anthropic", status, &body));
    }

    let mut text = String::new();
    let mut reasoning = String::new();
    let mut usage = ChatStreamUsage::default();
    let chunk_timeout = resolve_stream_chunk_timeout(options);
    let mut sse_buffer = String::new();
    let mut done = false;

    while !done {
        let Some(payloads) =
            read_next_sse_payload_batch(&mut response, chunk_timeout, &mut sse_buffer).await?
        else {
            break;
        };

        for payload in payloads {
            if payload.trim() == "[DONE]" {
                done = true;
                break;
            }

            let value: Value = serde_json::from_str(&payload).map_err(|error| {
                AppError::new(
                    ErrorCode::ExternalError,
                    format!("anthropic stream payload is invalid json: {error}"),
                )
            })?;

            if let Some(error) = value.get("error") {
                return Err(normalize_provider_stream_error("anthropic", error));
            }
            if value.get("type").and_then(Value::as_str) == Some("error") {
                return Err(normalize_provider_stream_error("anthropic", &value));
            }
            if is_sensitive_content_json(&value) {
                return Err(AppError::new(
                    ErrorCode::SensitiveContent,
                    "SENSITIVE_CONTENT: content rejected by provider safety policy",
                ));
            }

            let (prompt_tokens, completion_tokens) = extract_anthropic_usage_tokens(&value);
            if let Some(tokens) = prompt_tokens {
                usage.prompt_tokens = tokens;
            }
            if let Some(tokens) = completion_tokens {
                usage.completion_tokens = tokens;
            }

            let (text_piece, reasoning_piece) = extract_anthropic_stream_parts(&value);
            emit_stream_piece(
                &mut reasoning,
                reasoning_piece,
                ChatStreamChunkKind::Reasoning,
                on_chunk,
            )
            .await?;
            emit_stream_piece(&mut text, text_piece, ChatStreamChunkKind::Text, on_chunk).await?;

            if value.get("type").and_then(Value::as_str) == Some("message_stop") {
                done = true;
                break;
            }
        }
    }

    Ok(ChatStreamResult {
        text,
        reasoning,
        usage,
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_provider_stream<F, Fut>(
    provider_key: &str,
    provider_id: &str,
    provider_base_url: Option<&str>,
    provider_api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    options: &ChatStreamOptions,
    reasoning_enabled: bool,
    on_chunk: &mut F,
) -> Result<ChatStreamResult, AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    match provider_key {
        "openai-compatible" => {
            let base_url = provider_base_url
                .ok_or_else(|| AppError::invalid_params("llm provider baseUrl is required"))?;
            stream_with_openai_compatible(
                base_url,
                provider_api_key,
                model_id,
                messages,
                options,
                reasoning_enabled,
                on_chunk,
            )
            .await
        }
        "gemini-compatible" | "google" => {
            let base_url = provider_base_url
                .ok_or_else(|| AppError::invalid_params("gemini provider baseUrl is required"))?;
            stream_with_gemini_compatible(
                base_url,
                provider_api_key,
                model_id,
                messages,
                options,
                reasoning_enabled,
                on_chunk,
            )
            .await
        }
        "anthropic" => {
            let base_url = provider_base_url.ok_or_else(|| {
                AppError::invalid_params("anthropic provider baseUrl is required")
            })?;
            stream_with_anthropic(
                base_url,
                provider_api_key,
                model_id,
                messages,
                options,
                reasoning_enabled,
                on_chunk,
            )
            .await
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported llm provider: {provider_id}",
        ))),
    }
}

fn parse_openai_message_text(choice: &Value) -> String {
    let content = choice
        .get("message")
        .and_then(|item| item.get("content"))
        .cloned()
        .unwrap_or(Value::Null);

    if let Some(text) = content.as_str() {
        return text.to_string();
    }

    if let Some(parts) = content.as_array() {
        return parts
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .map(|item| item.to_string())
            })
            .collect::<Vec<_>>()
            .join("");
    }

    String::new()
}

pub async fn chat_completion_stream<F, Fut>(
    pool: &MySqlPool,
    model_key: &str,
    messages: &[ChatMessage],
    options: ChatStreamOptions,
    mut on_chunk: F,
) -> Result<ChatStreamResult, AppError>
where
    F: FnMut(ChatStreamChunk) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Llm)).await?;

    let emitted_chunks = AtomicUsize::new(0usize);
    let mut emit = |chunk: ChatStreamChunk| {
        emitted_chunks.fetch_add(1, Ordering::Relaxed);
        on_chunk(chunk)
    };

    let first = run_provider_stream(
        provider.provider_key.as_str(),
        provider.id.as_str(),
        provider.base_url.as_deref(),
        provider.api_key.as_str(),
        model.model_id.as_str(),
        messages,
        &options,
        options.reasoning,
        &mut emit,
    )
    .await;

    if !options.reasoning {
        return first;
    }

    match first {
        Ok(result) => {
            if result.text.trim().is_empty() && emitted_chunks.load(Ordering::Relaxed) == 0 {
                return run_provider_stream(
                    provider.provider_key.as_str(),
                    provider.id.as_str(),
                    provider.base_url.as_deref(),
                    provider.api_key.as_str(),
                    model.model_id.as_str(),
                    messages,
                    &options,
                    false,
                    &mut emit,
                )
                .await;
            }
            Ok(result)
        }
        Err(error) => {
            if emitted_chunks.load(Ordering::Relaxed) == 0 && should_retry_without_reasoning(&error)
            {
                return run_provider_stream(
                    provider.provider_key.as_str(),
                    provider.id.as_str(),
                    provider.base_url.as_deref(),
                    provider.api_key.as_str(),
                    model.model_id.as_str(),
                    messages,
                    &options,
                    false,
                    &mut emit,
                )
                .await;
            }
            Err(error)
        }
    }
}

pub async fn chat_completion(
    pool: &MySqlPool,
    model_key: &str,
    messages: &[ChatMessage],
    temperature: Option<f32>,
) -> Result<String, AppError> {
    let stream_result = chat_completion_stream(
        pool,
        model_key,
        messages,
        ChatStreamOptions {
            temperature,
            ..ChatStreamOptions::default()
        },
        |_chunk| async { Ok(()) },
    )
    .await?;

    Ok(stream_result.text)
}

pub async fn vision_completion(
    pool: &MySqlPool,
    model_key: &str,
    prompt: &str,
    image_sources: &[String],
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Llm)).await?;

    match provider.provider_key.as_str() {
        "openai-compatible" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("llm provider baseUrl is required"))?;
            let mut content = vec![json!({ "type": "text", "text": prompt })];
            for source in image_sources {
                content.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": media::normalize_source_to_data_url(source).await?
                    }
                }));
            }
            let payload = json!({
                "model": model.model_id,
                "messages": [{ "role": "user", "content": content }],
            });

            let response = http_client()
                .post(openai_chat_completions_endpoint(base_url))
                .bearer_auth(provider.api_key)
                .json(&payload)
                .send()
                .await
                .map_err(|err| AppError::internal(format!("vision request failed: {err}")))?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AppError::internal(format!(
                    "vision request failed ({status}): {body}"
                )));
            }
            let value: Value = response
                .json()
                .await
                .map_err(|err| AppError::internal(format!("invalid vision response: {err}")))?;
            let choice = value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .ok_or_else(|| AppError::internal("vision response missing choices"))?;
            Ok(parse_openai_message_text(choice))
        }
        "gemini-compatible" | "google" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("gemini provider baseUrl is required"))?;

            let endpoint = gemini_generate_content_endpoint(base_url, &model.model_id, false);

            let mut parts = Vec::with_capacity(image_sources.len() + 1);
            for source in image_sources {
                let data_url = media::normalize_source_to_data_url(source).await?;
                let (mime_type, data) = media::parse_data_url(&data_url)
                    .ok_or_else(|| AppError::invalid_params("invalid image data url"))?;
                parts.push(json!({
                    "inlineData": { "mimeType": mime_type, "data": data }
                }));
            }
            parts.push(json!({ "text": prompt }));

            let payload = json!({
                "contents": [{ "parts": parts }],
            });

            let response = http_client()
                .post(endpoint)
                .header("x-goog-api-key", provider.api_key.clone())
                .bearer_auth(provider.api_key)
                .json(&payload)
                .send()
                .await
                .map_err(|err| {
                    AppError::internal(format!("gemini vision request failed: {err}"))
                })?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AppError::internal(format!(
                    "gemini vision request failed ({status}): {body}"
                )));
            }

            let value: Value = response.json().await.map_err(|err| {
                AppError::internal(format!("invalid gemini vision response: {err}"))
            })?;

            Ok(value
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("content"))
                .and_then(|item| item.get("parts"))
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default())
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported llm provider: {}",
            provider.id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        extract_anthropic_stream_parts, extract_anthropic_usage_tokens, extract_openai_delta_parts,
        extract_sse_data, is_reasoning_unsupported_message, is_sensitive_content_json,
        merge_stream_piece,
    };

    #[test]
    fn extract_sse_data_merges_multiline_payload() {
        let payload = extract_sse_data("event: message\ndata: {\"a\":1}\ndata: {\"b\":2}\n")
            .expect("payload");
        assert_eq!(payload, "{\"a\":1}\n{\"b\":2}");
    }

    #[test]
    fn extract_openai_delta_parts_reads_text_and_reasoning() {
        let chunk = json!({
            "choices": [{
                "delta": {
                    "content": [
                        {"type": "reasoning", "text": "step-1"},
                        {"type": "text", "text": "answer"}
                    ]
                }
            }]
        });

        let (text, reasoning) = extract_openai_delta_parts(&chunk);
        assert_eq!(text, "answer");
        assert_eq!(reasoning, "step-1");
    }

    #[test]
    fn extract_anthropic_stream_parts_reads_text_and_thinking() {
        let text_chunk = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "hello"
            }
        });
        let thinking_chunk = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "thinking_delta",
                "thinking": "let me think"
            }
        });

        let (text, reasoning) = extract_anthropic_stream_parts(&text_chunk);
        assert_eq!(text, "hello");
        assert_eq!(reasoning, "");

        let (text, reasoning) = extract_anthropic_stream_parts(&thinking_chunk);
        assert_eq!(text, "");
        assert_eq!(reasoning, "let me think");
    }

    #[test]
    fn extract_anthropic_usage_tokens_reads_input_and_output() {
        let message_start = json!({
            "type": "message_start",
            "message": {
                "usage": {
                    "input_tokens": 123
                }
            }
        });
        let message_delta = json!({
            "type": "message_delta",
            "usage": {
                "output_tokens": 45
            }
        });

        let (prompt, completion) = extract_anthropic_usage_tokens(&message_start);
        assert_eq!(prompt, Some(123));
        assert_eq!(completion, None);

        let (prompt, completion) = extract_anthropic_usage_tokens(&message_delta);
        assert_eq!(prompt, None);
        assert_eq!(completion, Some(45));
    }

    #[test]
    fn merge_stream_piece_handles_cumulative_updates() {
        let mut text = String::new();

        assert_eq!(merge_stream_piece(&mut text, "hel"), "hel");
        assert_eq!(merge_stream_piece(&mut text, "hello"), "lo");
        assert_eq!(merge_stream_piece(&mut text, "hello"), "");
        assert_eq!(text, "hello");
    }

    #[test]
    fn sensitive_content_detection_handles_gemini_finish_reason() {
        let payload = json!({
            "candidates": [{
                "finishReason": "SAFETY"
            }]
        });

        assert!(is_sensitive_content_json(&payload));
    }

    #[test]
    fn reasoning_unsupported_message_detection_is_case_insensitive() {
        let message = "Unknown field: reasoning_effort is not supported by this model";
        assert!(is_reasoning_unsupported_message(message));
    }
}
