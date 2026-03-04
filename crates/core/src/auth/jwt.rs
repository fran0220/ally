use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{config::AppConfig, errors::AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    ttl_seconds: i64,
}

impl JwtService {
    pub fn from_config(config: &AppConfig) -> Self {
        let secret = config.jwt_secret.as_bytes();
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            ttl_seconds: config.jwt_ttl_seconds,
        }
    }

    pub fn issue_token(
        &self,
        user_id: &str,
        username: &str,
        role: &str,
    ) -> Result<String, AppError> {
        if user_id.trim().is_empty() {
            return Err(AppError::invalid_params("user id cannot be empty"));
        }
        if username.trim().is_empty() {
            return Err(AppError::invalid_params("username cannot be empty"));
        }

        let now = Utc::now();
        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.ttl_seconds)).timestamp(),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|err| AppError::internal(format!("failed to sign jwt: {err}")))
    }

    pub fn verify_token(&self, token: &str) -> Result<AccessTokenClaims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|err| AppError::unauthorized(format!("invalid token: {err}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn jwt_issue_and_verify_roundtrip() {
        let config = AppConfig {
            host: "127.0.0.1".to_string(),
            port: 3001,
            database_url: "mysql://root:pass@127.0.0.1:3306/test".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            jwt_secret: "secret".to_string(),
            api_encryption_key: String::new(),
            jwt_ttl_seconds: 3600,
            cors_allow_origin: vec![],
            internal_task_token: String::new(),
            billing_mode: crate::billing::BillingMode::Off,
            ark_api_key: String::new(),
            google_ai_key: String::new(),
            minimax_api_key: String::new(),
            vidu_api_key: String::new(),
            ark_api_base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
            google_api_base_url: "https://generativelanguage.googleapis.com".to_string(),
            minimax_api_base_url: "https://api.minimaxi.com/v1".to_string(),
            vidu_api_base_url: "https://api.vidu.cn/ent/v2".to_string(),
            generator_http_timeout_secs: 120,
            llm_stream_chunk_timeout_ms: 180_000,
            generator_poll_interval_ms: 3_000,
            generator_poll_timeout_secs: 1_200,
            generator_retry_max_attempts: 3,
            generator_retry_backoff_ms: 1_000,
        };

        let service = JwtService::from_config(&config);
        let token = service
            .issue_token("user-1", "alice", "user")
            .expect("token should be issued");

        let claims = service.verify_token(&token).expect("token should verify");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "user");
    }
}
