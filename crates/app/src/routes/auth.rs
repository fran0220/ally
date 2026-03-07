use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySql;
use uuid::Uuid;
use waoowaoo_core::auth::{hash_password, verify_password};

use crate::error::AppError;

use crate::{app_state::AppState, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
struct AuthUserResponse {
    id: String,
    name: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    token: String,
    user: AuthUserResponse,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    name: String,
    role: String,
    password: Option<String>,
}

const AUTH_COOKIE_NAME: &str = "token";
const AUTH_COOKIE_PATH: &str = "/";
const AUTH_COOKIE_SAME_SITE: &str = "Lax";
const AUTH_COOKIE_EXPIRES_AT_EPOCH: &str = "Thu, 01 Jan 1970 00:00:00 GMT";

fn insert_set_cookie(headers: &mut HeaderMap, cookie: &str) -> Result<(), AppError> {
    let value = header::HeaderValue::from_str(cookie)
        .map_err(|err| AppError::internal(format!("failed to build auth cookie: {err}")))?;
    headers.insert(header::SET_COOKIE, value);
    Ok(())
}

fn token_cookie(token: &str) -> String {
    format!(
        "{AUTH_COOKIE_NAME}={token}; HttpOnly; Path={AUTH_COOKIE_PATH}; SameSite={AUTH_COOKIE_SAME_SITE}"
    )
}

fn cleared_token_cookie() -> String {
    format!(
        "{AUTH_COOKIE_NAME}=; HttpOnly; Path={AUTH_COOKIE_PATH}; SameSite={AUTH_COOKIE_SAME_SITE}; Max-Age=0; Expires={AUTH_COOKIE_EXPIRES_AT_EPOCH}"
    )
}

fn set_token_cookie(headers: &mut HeaderMap, token: &str) -> Result<(), AppError> {
    insert_set_cookie(headers, &token_cookie(token))
}

fn clear_token_cookie(headers: &mut HeaderMap) -> Result<(), AppError> {
    insert_set_cookie(headers, &cleared_token_cookie())
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_params("name is required"));
    }
    if payload.password.len() < 6 {
        return Err(AppError::invalid_params(
            "password must be at least 6 chars",
        ));
    }

    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM user WHERE name = ? LIMIT 1")
        .bind(name)
        .fetch_optional(&state.mysql)
        .await?;
    if existing.is_some() {
        return Err(AppError::conflict("user already exists"));
    }

    let hash = hash_password(&payload.password)?;
    let user_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO user (id, name, password, role, createdAt, updatedAt) VALUES (?, ?, ?, 'user', NOW(3), NOW(3))",
    )
    .bind(&user_id)
    .bind(name)
    .bind(hash)
    .execute(&state.mysql)
    .await?;

    let token = state.jwt.issue_token(&user_id, name, "user")?;

    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    let response = Json(json!({
        "message": "注册成功",
        "token": token,
        "user": {
            "id": user_id,
            "name": name,
            "role": "user"
        }
    }));

    Ok((StatusCode::CREATED, headers, response))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.username.trim().is_empty() || payload.password.is_empty() {
        return Err(AppError::invalid_params(
            "username and password are required",
        ));
    }

    let user = sqlx::query_as::<MySql, UserRow>(
        "SELECT id, name, role, password FROM user WHERE name = ? LIMIT 1",
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

    let hash = user
        .password
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

    let verified = verify_password(&payload.password, hash)?;
    if !verified {
        return Err(AppError::unauthorized("invalid credentials"));
    }

    let token = state.jwt.issue_token(&user.id, &user.name, &user.role)?;

    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    let response = Json(AuthResponse {
        token,
        user: AuthUserResponse {
            id: user.id,
            name: user.name,
            role: user.role,
        },
    });

    Ok((StatusCode::OK, headers, response))
}

pub async fn refresh(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let token = state
        .jwt
        .issue_token(&user.id, &user.username, &user.role)?;
    let mut headers = HeaderMap::new();
    set_token_cookie(&mut headers, &token)?;

    Ok((
        StatusCode::OK,
        headers,
        Json(json!({
            "token": token,
            "user": {
                "id": user.id,
                "name": user.username,
                "role": user.role,
            }
        })),
    ))
}

pub async fn session(user: AuthUser) -> Result<impl IntoResponse, AppError> {
    Ok((
        StatusCode::OK,
        Json(json!({
            "user": {
                "id": user.id,
                "name": user.username,
                "role": user.role,
            }
        })),
    ))
}

pub async fn logout() -> Result<impl IntoResponse, AppError> {
    let mut headers = HeaderMap::new();
    clear_token_cookie(&mut headers)?;
    Ok((
        StatusCode::OK,
        headers,
        Json(json!({
            "success": true,
        })),
    ))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{HeaderMap, StatusCode, header},
        response::IntoResponse,
    };

    use super::{
        AUTH_COOKIE_EXPIRES_AT_EPOCH, clear_token_cookie, logout, session, set_token_cookie,
    };
    use crate::extractors::auth::AuthUser;

    #[test]
    fn set_token_cookie_sets_expected_scope() {
        let mut headers = HeaderMap::new();
        set_token_cookie(&mut headers, "jwt-token").expect("setting auth cookie should succeed");

        let set_cookie = headers
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("set-cookie should be present and valid utf-8");

        assert!(set_cookie.starts_with("token=jwt-token;"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("Path=/"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert!(!set_cookie.contains("Max-Age=0"));
    }

    #[test]
    fn clear_token_cookie_sets_expired_cookie_header() {
        let mut headers = HeaderMap::new();
        clear_token_cookie(&mut headers).expect("clearing auth cookie should succeed");

        let set_cookie = headers
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("set-cookie should be present and valid utf-8");

        assert!(set_cookie.starts_with("token=;"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("Path=/"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert!(set_cookie.contains("Max-Age=0"));
        assert!(set_cookie.contains(&format!("Expires={AUTH_COOKIE_EXPIRES_AT_EPOCH}")));
    }

    #[tokio::test]
    async fn logout_returns_ok_and_cookie_clear_header() {
        let response = logout()
            .await
            .expect("logout handler should succeed")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("logout response should include set-cookie");
        assert!(set_cookie.contains("Max-Age=0"));
        assert!(set_cookie.contains(&format!("Expires={AUTH_COOKIE_EXPIRES_AT_EPOCH}")));

        let (_, body) = response.into_parts();
        let body_bytes = to_bytes(body, usize::MAX)
            .await
            .expect("logout response body should be readable");
        let body_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("logout body should be valid json");
        assert_eq!(
            body_json
                .get("success")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[tokio::test]
    async fn session_returns_authenticated_user_payload() {
        let response = session(AuthUser {
            id: "user-1".to_string(),
            username: "alice".to_string(),
            role: "admin".to_string(),
        })
        .await
        .expect("session handler should succeed")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = to_bytes(body, usize::MAX)
            .await
            .expect("session response body should be readable");
        let body_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("session body should be valid json");

        assert_eq!(
            body_json
                .get("user")
                .and_then(|value| value.get("id"))
                .and_then(serde_json::Value::as_str),
            Some("user-1")
        );
        assert_eq!(
            body_json
                .get("user")
                .and_then(|value| value.get("name"))
                .and_then(serde_json::Value::as_str),
            Some("alice")
        );
        assert_eq!(
            body_json
                .get("user")
                .and_then(|value| value.get("role"))
                .and_then(serde_json::Value::as_str),
            Some("admin")
        );
    }
}
