use anyhow::{Context, Result};
use config::{Config, Environment, File};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    #[serde(default = "default_redis_url")]
    pub redis_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_api_encryption_key")]
    pub api_encryption_key: String,
    #[serde(default = "default_jwt_ttl_seconds")]
    pub jwt_ttl_seconds: i64,
    #[serde(default, deserialize_with = "deserialize_cors_allow_origin")]
    pub cors_allow_origin: Vec<String>,
    #[serde(default)]
    pub internal_task_token: String,
    #[serde(default = "default_billing_enabled")]
    pub billing_enabled: bool,
    #[serde(default)]
    pub ark_api_key: String,
    #[serde(default)]
    pub google_ai_key: String,
    #[serde(default)]
    pub minimax_api_key: String,
    #[serde(default)]
    pub vidu_api_key: String,
    #[serde(default = "default_ark_api_base_url")]
    pub ark_api_base_url: String,
    #[serde(default = "default_google_api_base_url")]
    pub google_api_base_url: String,
    #[serde(default = "default_minimax_api_base_url")]
    pub minimax_api_base_url: String,
    #[serde(default = "default_vidu_api_base_url")]
    pub vidu_api_base_url: String,
    #[serde(default = "default_generator_http_timeout_secs")]
    pub generator_http_timeout_secs: u64,
    #[serde(default = "default_llm_stream_chunk_timeout_ms")]
    pub llm_stream_chunk_timeout_ms: u64,
    #[serde(default = "default_generator_poll_interval_ms")]
    pub generator_poll_interval_ms: u64,
    #[serde(default = "default_generator_poll_timeout_secs")]
    pub generator_poll_timeout_secs: u64,
    #[serde(default = "default_generator_retry_max_attempts")]
    pub generator_retry_max_attempts: u32,
    #[serde(default = "default_generator_retry_backoff_ms")]
    pub generator_retry_backoff_ms: u64,
}

fn deserialize_cors_allow_origin<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawCorsAllowOrigin {
        List(Vec<String>),
        CommaSeparated(String),
    }

    match RawCorsAllowOrigin::deserialize(deserializer)? {
        RawCorsAllowOrigin::List(values) => Ok(values
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect()),
        RawCorsAllowOrigin::CommaSeparated(raw) => Ok(raw
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect()),
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    43001
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_jwt_ttl_seconds() -> i64 {
    60 * 60 * 24 * 7
}

fn default_api_encryption_key() -> String {
    String::new()
}

fn default_billing_enabled() -> bool {
    false
}

fn default_ark_api_base_url() -> String {
    "https://ark.cn-beijing.volces.com/api/v3".to_string()
}

fn default_google_api_base_url() -> String {
    "https://generativelanguage.googleapis.com".to_string()
}

fn default_minimax_api_base_url() -> String {
    "https://api.minimaxi.com/v1".to_string()
}

fn default_vidu_api_base_url() -> String {
    "https://api.vidu.cn/ent/v2".to_string()
}

fn default_generator_http_timeout_secs() -> u64 {
    120
}

fn default_llm_stream_chunk_timeout_ms() -> u64 {
    180_000
}

fn default_generator_poll_interval_ms() -> u64 {
    3_000
}

fn default_generator_poll_timeout_secs() -> u64 {
    20 * 60
}

fn default_generator_retry_max_attempts() -> u32 {
    3
}

fn default_generator_retry_backoff_ms() -> u64 {
    1_000
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let loaded = Config::builder()
            .add_source(File::with_name(".env").required(false))
            .add_source(
                Environment::default()
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("cors_allow_origin"),
            )
            .build()
            .context("failed to build runtime config")?;

        loaded
            .try_deserialize::<AppConfig>()
            .context("failed to deserialize runtime config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_stable() {
        assert_eq!(default_port(), 43001);
        assert_eq!(default_jwt_ttl_seconds(), 604_800);
        assert_eq!(default_api_encryption_key(), "");
        assert!(!default_billing_enabled());
        assert_eq!(default_llm_stream_chunk_timeout_ms(), 180_000);
        assert_eq!(default_generator_poll_interval_ms(), 3_000);
        assert_eq!(default_generator_poll_timeout_secs(), 1_200);
        assert_eq!(default_generator_retry_max_attempts(), 3);
        assert_eq!(default_generator_retry_backoff_ms(), 1_000);
    }
}
