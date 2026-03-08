use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    routing::get,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::{MySql, QueryBuilder};
use waoowaoo_core::billing::{
    BILLING_CURRENCY, TransactionListInput, decimal_to_f64, get_balance, get_project_cost_details,
    get_user_cost_details, get_user_cost_summary, list_user_transactions,
};

use crate::{app_state::AppState, error::AppError, extractors::auth::AuthUser};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CostDetailsQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionsQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    #[serde(default)]
    tx_type: Option<String>,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectNameRow {
    id: String,
    name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectOwnerRow {
    #[sqlx(rename = "userId")]
    user_id: String,
    name: String,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

fn normalize_page(page: i64) -> i64 {
    page.max(1)
}

fn normalize_page_size(page_size: i64) -> i64 {
    page_size.clamp(1, 200)
}

fn request_locale(headers: &HeaderMap) -> &'static str {
    let is_english = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|raw| raw.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .and_then(|raw| raw.split(';').next())
        .map(str::trim)
        .map(|raw| raw.to_ascii_lowercase())
        .filter(|raw| !raw.is_empty())
        .is_some_and(|raw| raw == "en" || raw.starts_with("en-"));
    if is_english { "en" } else { "zh" }
}

fn localized_msg<'a>(locale: &str, zh: &'a str, en: &'a str) -> &'a str {
    if locale == "en" { en } else { zh }
}

fn parse_start_datetime(raw: &str) -> Result<NaiveDateTime, AppError> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Ok(parsed.with_timezone(&Utc).naive_utc());
    }

    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .ok_or_else(|| AppError::invalid_params(format!("invalid startDate: {raw}")))
}

fn parse_end_datetime(raw: &str) -> Result<NaiveDateTime, AppError> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Ok(parsed.with_timezone(&Utc).naive_utc());
    }

    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_milli_opt(23, 59, 59, 999))
        .ok_or_else(|| AppError::invalid_params(format!("invalid endDate: {raw}")))
}

async fn fetch_project_name_map(
    state: &AppState,
    project_ids: &[String],
) -> Result<HashMap<String, String>, AppError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut qb: QueryBuilder<'_, MySql> =
        QueryBuilder::new("SELECT id, name FROM projects WHERE id IN (");
    {
        let mut separated = qb.separated(",");
        for project_id in project_ids {
            separated.push_bind(project_id);
        }
        separated.push_unseparated(")");
    }

    let rows = qb
        .build_query_as::<ProjectNameRow>()
        .fetch_all(&state.mysql)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect::<HashMap<_, _>>())
}

async fn balance(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let snapshot = get_balance(&state.mysql, &user.id).await?;

    Ok(Json(json!({
        "success": true,
        "currency": BILLING_CURRENCY,
        "balance": decimal_to_f64(snapshot.balance),
        "totalSpent": decimal_to_f64(snapshot.total_spent),
    })))
}

async fn costs(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let locale = request_locale(&headers);
    let summary = get_user_cost_summary(&state.mysql, &user.id).await?;

    let project_ids = summary
        .by_project
        .iter()
        .map(|item| item.project_id.clone())
        .collect::<Vec<_>>();
    let project_name_map = fetch_project_name_map(&state, &project_ids).await?;

    let mut by_project = summary
        .by_project
        .into_iter()
        .map(|item| {
            json!({
                "projectId": item.project_id,
                "projectName": project_name_map
                    .get(&item.project_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        localized_msg(locale, "未知项目", "Unknown Project").to_string()
                    }),
                "totalCost": item.total_cost,
                "recordCount": item.record_count,
            })
        })
        .collect::<Vec<_>>();

    by_project.sort_by(|left, right| {
        let left_cost = left
            .get("totalCost")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let right_cost = right
            .get("totalCost")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        right_cost
            .partial_cmp(&left_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Json(json!({
        "userId": user.id,
        "currency": BILLING_CURRENCY,
        "total": summary.total,
        "byProject": by_project,
    })))
}

async fn costs_details(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<CostDetailsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let details = get_user_cost_details(&state.mysql, &user.id, page, page_size).await?;

    Ok(Json(json!({
        "success": true,
        "currency": BILLING_CURRENCY,
        "records": details.records,
        "total": details.total,
        "page": details.page,
        "pageSize": details.page_size,
        "totalPages": details.total_pages,
    })))
}

async fn transactions(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<TransactionsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let start_at = query
        .start_date
        .as_deref()
        .map(parse_start_datetime)
        .transpose()?;
    let end_at = query
        .end_date
        .as_deref()
        .map(parse_end_datetime)
        .transpose()?;

    let result = list_user_transactions(
        &state.mysql,
        &TransactionListInput {
            user_id: user.id,
            page,
            page_size,
            tx_type: query.tx_type,
            start_at,
            end_at,
        },
    )
    .await?;

    Ok(Json(json!({
        "currency": BILLING_CURRENCY,
        "transactions": result.transactions,
        "pagination": {
            "page": page,
            "pageSize": page_size,
            "total": result.total,
            "totalPages": ((result.total + page_size - 1) / page_size).max(1),
        }
    })))
}

async fn project_costs(
    State(state): State<AppState>,
    user: AuthUser,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project = sqlx::query_as::<_, ProjectOwnerRow>(
        "SELECT userId, name FROM projects WHERE id = ? LIMIT 1",
    )
    .bind(&project_id)
    .fetch_optional(&state.mysql)
    .await?
    .ok_or_else(|| AppError::not_found("project not found"))?;

    if project.user_id != user.id {
        return Err(AppError::forbidden("project access denied"));
    }

    let details = get_project_cost_details(&state.mysql, &project_id).await?;
    Ok(Json(json!({
        "projectId": project_id,
        "projectName": project.name,
        "currency": BILLING_CURRENCY,
        "total": details.total,
        "byType": details.by_type,
        "byAction": details.by_action,
        "recentRecords": details.recent_records,
    })))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/user/balance", get(balance))
        .route("/api/user/costs", get(costs))
        .route("/api/user/costs/details", get(costs_details))
        .route("/api/user/transactions", get(transactions))
        .route("/api/projects/{id}/costs", get(project_costs))
}
