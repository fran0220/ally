use crate::error::AppError;
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use serde::Serialize;

use crate::app_state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUser(pub AuthUser);

fn parse_bearer_token(raw: &str) -> Option<String> {
    let mut segments = raw.splitn(2, ' ');
    let scheme = segments.next()?.trim();
    let credentials = segments.next()?.trim();
    if !scheme.eq_ignore_ascii_case("bearer") || credentials.is_empty() {
        return None;
    }
    Some(credentials.to_string())
}

fn parse_token(parts: &Parts) -> Result<Option<String>, AppError> {
    if let Some(value) = parts.headers.get(header::AUTHORIZATION) {
        let raw = value
            .to_str()
            .map_err(|_| AppError::unauthorized("invalid authorization header"))?
            .trim();
        let token = parse_bearer_token(raw)
            .ok_or_else(|| AppError::unauthorized("invalid authorization header"))?;
        return Ok(Some(token));
    }

    if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
        let raw = cookie_header
            .to_str()
            .map_err(|_| AppError::unauthorized("invalid cookie header"))?;
        for segment in raw.split(';') {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();
            if key == "token" && !value.is_empty() {
                return Ok(Some(value.to_string()));
            }
        }
    }

    Ok(None)
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = parse_token(parts)? else {
            return Err(AppError::unauthorized("missing auth token"));
        };

        let claims = state.jwt.verify_token(&token)?;
        Ok(Self {
            id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "admin" {
            return Err(AppError::forbidden("admin role required"));
        }
        Ok(Self(user))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{Request, header};

    use super::parse_token;

    fn request_parts(headers: Vec<(&str, &str)>) -> axum::http::request::Parts {
        let mut request_builder = Request::builder().uri("/");
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }

        let request = request_builder
            .body(())
            .expect("request should build for token parsing tests");
        let (parts, _) = request.into_parts();
        parts
    }

    #[test]
    fn parse_token_returns_none_when_no_auth_headers_present() {
        let parts = request_parts(vec![]);
        let token = parse_token(&parts).expect("parsing should succeed");
        assert!(token.is_none());
    }

    #[test]
    fn parse_token_reads_bearer_authorization_header() {
        let parts = request_parts(vec![(header::AUTHORIZATION.as_str(), "Bearer token-123")]);
        let token = parse_token(&parts).expect("parsing should succeed");
        assert_eq!(token.as_deref(), Some("token-123"));
    }

    #[test]
    fn parse_token_rejects_invalid_authorization_scheme() {
        let parts = request_parts(vec![(header::AUTHORIZATION.as_str(), "Token token-123")]);
        let error = parse_token(&parts).expect_err("invalid scheme should be rejected");
        assert!(
            error.to_string().contains("invalid authorization header"),
            "unexpected error message: {error}"
        );
    }

    #[test]
    fn parse_token_reads_cookie_when_authorization_missing() {
        let parts = request_parts(vec![(
            header::COOKIE.as_str(),
            "a=1; token=cookie-token; b=2",
        )]);
        let token = parse_token(&parts).expect("parsing should succeed");
        assert_eq!(token.as_deref(), Some("cookie-token"));
    }

    #[test]
    fn parse_token_ignores_malformed_cookie_segments() {
        let parts = request_parts(vec![(header::COOKIE.as_str(), "broken; token=from-cookie")]);
        let token = parse_token(&parts).expect("parsing should succeed");
        assert_eq!(token.as_deref(), Some("from-cookie"));
    }
}
