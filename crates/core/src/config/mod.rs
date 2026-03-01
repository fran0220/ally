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
    #[serde(default = "default_jwt_ttl_seconds")]
    pub jwt_ttl_seconds: i64,
    #[serde(default, deserialize_with = "deserialize_cors_allow_origin")]
    pub cors_allow_origin: Vec<String>,
    #[serde(default)]
    pub internal_task_token: String,
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
    3001
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_jwt_ttl_seconds() -> i64 {
    60 * 60 * 24 * 7
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
        assert_eq!(default_port(), 3001);
        assert_eq!(default_jwt_ttl_seconds(), 604_800);
    }
}
