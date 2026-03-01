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

fn parse_token(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(header::AUTHORIZATION) {
        let raw = value.to_str().ok()?.trim();
        if let Some(token) = raw.strip_prefix("Bearer ")
            && !token.trim().is_empty()
        {
            return Some(token.trim().to_string());
        }
    }

    if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
        let raw = cookie_header.to_str().ok()?;
        for segment in raw.split(';') {
            let mut kv = segment.trim().splitn(2, '=');
            let key = kv.next()?.trim();
            let value = kv.next()?.trim();
            if key == "token" && !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = parse_token(parts) else {
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
