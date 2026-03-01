use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::{
    api_config::UnifiedModelType, errors::AppError, media, runtime::resolve_model_with_provider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

fn http_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
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

async fn chat_with_openai_compatible(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    temperature: Option<f32>,
) -> Result<String, AppError> {
    let payload = json!({
        "model": model_id,
        "messages": messages,
        "temperature": temperature.unwrap_or(0.7),
    });

    let response = http_client()
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("llm request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "llm request failed ({status}): {body}"
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid llm response: {err}")))?;

    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| AppError::internal("llm response missing choices"))?;

    Ok(parse_openai_message_text(choice))
}

async fn chat_with_gemini_compatible(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    temperature: Option<f32>,
) -> Result<String, AppError> {
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

    let endpoint = if base_url.contains("/v1beta") {
        format!(
            "{}/models/{}:generateContent",
            base_url.trim_end_matches('/'),
            model_id
        )
    } else {
        format!(
            "{}/v1beta/models/{}:generateContent",
            base_url.trim_end_matches('/'),
            model_id
        )
    };

    let payload = json!({
        "contents": contents,
        "config": {
            "temperature": temperature.unwrap_or(0.7),
            "systemInstruction": if system_prompt.trim().is_empty() {
                Value::Null
            } else {
                json!({
                    "parts": [{ "text": system_prompt }]
                })
            }
        }
    });

    let response = http_client()
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("gemini request failed: {err}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "gemini request failed ({status}): {body}"
        )));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("invalid gemini response: {err}")))?;

    let text = value
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
        .unwrap_or_default();

    Ok(text)
}

pub async fn chat_completion(
    pool: &MySqlPool,
    model_key: &str,
    messages: &[ChatMessage],
    temperature: Option<f32>,
) -> Result<String, AppError> {
    let (model, provider) =
        resolve_model_with_provider(pool, model_key, Some(UnifiedModelType::Llm)).await?;

    match provider.provider_key.as_str() {
        "openai-compatible" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("llm provider baseUrl is required"))?;
            chat_with_openai_compatible(
                base_url,
                &provider.api_key,
                &model.model_id,
                messages,
                temperature,
            )
            .await
        }
        "gemini-compatible" | "google" => {
            let base_url = provider
                .base_url
                .as_deref()
                .ok_or_else(|| AppError::invalid_params("gemini provider baseUrl is required"))?;
            chat_with_gemini_compatible(
                base_url,
                &provider.api_key,
                &model.model_id,
                messages,
                temperature,
            )
            .await
        }
        _ => Err(AppError::invalid_params(format!(
            "unsupported llm provider: {}",
            provider.id
        ))),
    }
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
                .post(format!(
                    "{}/chat/completions",
                    base_url.trim_end_matches('/')
                ))
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

            let endpoint = if base_url.contains("/v1beta") {
                format!(
                    "{}/models/{}:generateContent",
                    base_url.trim_end_matches('/'),
                    model.model_id
                )
            } else {
                format!(
                    "{}/v1beta/models/{}:generateContent",
                    base_url.trim_end_matches('/'),
                    model.model_id
                )
            };

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
