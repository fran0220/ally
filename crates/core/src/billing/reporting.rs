use chrono::NaiveDateTime;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde::Serialize;
use serde_json::Value;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::errors::AppError;

use super::money::decimal_to_f64;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCostBreakdown {
    pub project_id: String,
    pub total_cost: f64,
    pub record_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCostSummary {
    pub total: f64,
    pub by_project: Vec<ProjectCostBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCostDetailRecord {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub api_type: String,
    pub model: String,
    pub action: String,
    pub quantity: i32,
    pub unit: String,
    pub cost: f64,
    pub metadata: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCostDetailsPage {
    pub records: Vec<UsageCostDetailRecord>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCostRecentRecord {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub api_type: String,
    pub model: String,
    pub action: String,
    pub quantity: i32,
    pub unit: String,
    pub cost: f64,
    pub metadata: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCostDetails {
    pub total: f64,
    pub by_type: Vec<ProjectCostBreakdownItem>,
    pub by_action: Vec<ProjectCostBreakdownItem>,
    pub recent_records: Vec<ProjectCostRecentRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCostBreakdownItem {
    pub key: String,
    pub total_cost: f64,
    pub record_count: i64,
}

#[derive(Debug, Clone)]
pub struct TransactionListInput {
    pub user_id: String,
    pub page: i64,
    pub page_size: i64,
    pub tx_type: Option<String>,
    pub start_at: Option<NaiveDateTime>,
    pub end_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTransactionRecord {
    pub id: String,
    pub user_id: String,
    pub tx_type: String,
    pub amount: f64,
    pub balance_after: f64,
    pub description: Option<String>,
    pub related_id: Option<String>,
    pub freeze_id: Option<String>,
    pub operator_id: Option<String>,
    pub external_order_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub episode_id: Option<String>,
    pub episode_number: Option<i32>,
    pub episode_name: Option<String>,
    pub task_type: Option<String>,
    pub action: Option<String>,
    pub billing_meta: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionListResult {
    pub transactions: Vec<UserTransactionRecord>,
    pub total: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct CostAggregateRow {
    key: String,
    total_cost: Decimal,
    record_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct UsageCostRow {
    id: String,
    project_id: String,
    user_id: String,
    api_type: String,
    model: String,
    action: String,
    quantity: Decimal,
    unit: String,
    cost: Decimal,
    metadata: Option<sqlx::types::Json<Value>>,
    created_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct TotalRow {
    total: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct TransactionRow {
    id: String,
    user_id: String,
    tx_type: String,
    amount: Decimal,
    balance_after: Decimal,
    description: Option<String>,
    related_id: Option<String>,
    freeze_id: Option<String>,
    operator_id: Option<String>,
    external_order_id: Option<String>,
    idempotency_key: Option<String>,
    project_id: Option<String>,
    project_name: Option<String>,
    episode_id: Option<String>,
    episode_number: Option<i32>,
    episode_name: Option<String>,
    action: Option<String>,
    billing_meta: Option<sqlx::types::Json<Value>>,
    created_at: NaiveDateTime,
}

fn decimal_quantity_to_i32(value: Decimal) -> i32 {
    value.trunc().to_i32().unwrap_or(0)
}

pub async fn get_project_total_cost(pool: &MySqlPool, project_id: &str) -> Result<f64, AppError> {
    let total = sqlx::query_as::<_, TotalRow>(
        "SELECT COALESCE(SUM(amount), 0) AS total FROM credit_records WHERE project_id = ? AND type = 'consume'",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    Ok(decimal_to_f64(total.total))
}

pub async fn get_project_cost_details(
    pool: &MySqlPool,
    project_id: &str,
) -> Result<ProjectCostDetails, AppError> {
    let by_type_raw = sqlx::query_as::<_, CostAggregateRow>(
        "SELECT api_type AS `key`, COALESCE(SUM(amount), 0) AS total_cost, COUNT(*) AS record_count FROM credit_records WHERE project_id = ? AND type = 'consume' AND api_type IS NOT NULL GROUP BY api_type",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let by_action_raw = sqlx::query_as::<_, CostAggregateRow>(
        "SELECT action AS `key`, COALESCE(SUM(amount), 0) AS total_cost, COUNT(*) AS record_count FROM credit_records WHERE project_id = ? AND type = 'consume' AND action IS NOT NULL GROUP BY action",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let recent = sqlx::query_as::<_, UsageCostRow>(
        "SELECT id, COALESCE(project_id, '') AS project_id, user_id, COALESCE(api_type, '') AS api_type, COALESCE(model, '') AS model, COALESCE(action, '') AS action, COALESCE(quantity, 0) AS quantity, COALESCE(unit, '') AS unit, amount AS cost, metadata, created_at FROM credit_records WHERE project_id = ? AND type = 'consume' ORDER BY created_at DESC LIMIT 50",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let by_type = by_type_raw
        .into_iter()
        .map(|row| ProjectCostBreakdownItem {
            key: row.key,
            total_cost: decimal_to_f64(row.total_cost),
            record_count: row.record_count,
        })
        .collect::<Vec<_>>();
    let by_action = by_action_raw
        .into_iter()
        .map(|row| ProjectCostBreakdownItem {
            key: row.key,
            total_cost: decimal_to_f64(row.total_cost),
            record_count: row.record_count,
        })
        .collect::<Vec<_>>();
    let recent_records = recent
        .into_iter()
        .map(|row| ProjectCostRecentRecord {
            id: row.id,
            project_id: row.project_id,
            user_id: row.user_id,
            api_type: row.api_type,
            model: row.model,
            action: row.action,
            quantity: decimal_quantity_to_i32(row.quantity),
            unit: row.unit,
            cost: decimal_to_f64(row.cost),
            metadata: row.metadata.map(|value| value.0.to_string()),
            created_at: row.created_at,
        })
        .collect::<Vec<_>>();

    Ok(ProjectCostDetails {
        total: get_project_total_cost(pool, project_id).await?,
        by_type,
        by_action,
        recent_records,
    })
}

pub async fn get_user_cost_summary(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<UserCostSummary, AppError> {
    let by_project = sqlx::query_as::<_, CostAggregateRow>(
        "SELECT project_id AS `key`, COALESCE(SUM(amount), 0) AS total_cost, COUNT(*) AS record_count FROM credit_records WHERE user_id = ? AND type = 'consume' AND project_id IS NOT NULL GROUP BY project_id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_as::<_, TotalRow>(
        "SELECT COALESCE(SUM(amount), 0) AS total FROM credit_records WHERE user_id = ? AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(UserCostSummary {
        total: decimal_to_f64(total.total),
        by_project: by_project
            .into_iter()
            .map(|row| ProjectCostBreakdown {
                project_id: row.key,
                total_cost: decimal_to_f64(row.total_cost),
                record_count: row.record_count,
            })
            .collect(),
    })
}

pub async fn get_user_cost_details(
    pool: &MySqlPool,
    user_id: &str,
    page: i64,
    page_size: i64,
) -> Result<UserCostDetailsPage, AppError> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 200);
    let offset = (page - 1) * page_size;

    let rows = sqlx::query_as::<_, UsageCostRow>(
        "SELECT id, COALESCE(project_id, '') AS project_id, user_id, COALESCE(api_type, '') AS api_type, COALESCE(model, '') AS model, COALESCE(action, '') AS action, COALESCE(quantity, 0) AS quantity, COALESCE(unit, '') AS unit, amount AS cost, metadata, created_at FROM credit_records WHERE user_id = ? AND type = 'consume' ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM credit_records WHERE user_id = ? AND type = 'consume'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(UserCostDetailsPage {
        records: rows
            .into_iter()
            .map(|row| UsageCostDetailRecord {
                id: row.id,
                project_id: row.project_id,
                user_id: row.user_id,
                api_type: row.api_type,
                model: row.model,
                action: row.action,
                quantity: decimal_quantity_to_i32(row.quantity),
                unit: row.unit,
                cost: decimal_to_f64(row.cost),
                metadata: row.metadata.map(|value| value.0.to_string()),
                created_at: row.created_at,
            })
            .collect(),
        total,
        page,
        page_size,
        total_pages: ((total + page_size - 1) / page_size).max(1),
    })
}

fn push_transaction_filters<'a>(qb: &mut QueryBuilder<'a, MySql>, input: &'a TransactionListInput) {
    if let Some(tx_type) = input
        .tx_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "all")
    {
        qb.push(" AND cr.type = ");
        qb.push_bind(tx_type);
    }

    if let Some(start_at) = input.start_at {
        qb.push(" AND cr.created_at >= ");
        qb.push_bind(start_at);
    }
    if let Some(end_at) = input.end_at {
        qb.push(" AND cr.created_at <= ");
        qb.push_bind(end_at);
    }
}

pub async fn list_user_transactions(
    pool: &MySqlPool,
    input: &TransactionListInput,
) -> Result<TransactionListResult, AppError> {
    let page = input.page.max(1);
    let page_size = input.page_size.clamp(1, 200);
    let offset = (page - 1) * page_size;

    let mut count_qb: QueryBuilder<'_, MySql> =
        QueryBuilder::new("SELECT COUNT(*) AS total FROM credit_records cr WHERE cr.user_id = ");
    count_qb.push_bind(&input.user_id);
    push_transaction_filters(&mut count_qb, input);

    let total = count_qb.build_query_scalar::<i64>().fetch_one(pool).await?;

    let mut data_qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT cr.id, cr.user_id, cr.type AS tx_type, cr.amount, cr.balance_after, NULL AS description, NULL AS related_id, NULL AS freeze_id, cr.operator_id, cr.external_order_id, cr.idempotency_key, cr.project_id, p.name AS project_name, cr.episode_id, e.episodeNumber AS episode_number, e.name AS episode_name, cr.action, cr.metadata AS billing_meta, cr.created_at FROM credit_records cr LEFT JOIN projects p ON p.id = cr.project_id LEFT JOIN novel_promotion_episodes e ON e.id = cr.episode_id WHERE cr.user_id = ",
    );
    data_qb.push_bind(&input.user_id);
    push_transaction_filters(&mut data_qb, input);
    data_qb.push(" ORDER BY cr.created_at DESC LIMIT ");
    data_qb.push_bind(page_size);
    data_qb.push(" OFFSET ");
    data_qb.push_bind(offset);

    let rows = data_qb
        .build_query_as::<TransactionRow>()
        .fetch_all(pool)
        .await?;

    let transactions = rows
        .into_iter()
        .map(|row| UserTransactionRecord {
            id: row.id,
            user_id: row.user_id,
            tx_type: row.tx_type,
            amount: decimal_to_f64(row.amount),
            balance_after: decimal_to_f64(row.balance_after),
            description: row.description,
            related_id: row.related_id,
            freeze_id: row.freeze_id,
            operator_id: row.operator_id,
            external_order_id: row.external_order_id,
            idempotency_key: row.idempotency_key,
            project_id: row.project_id,
            project_name: row.project_name,
            episode_id: row.episode_id,
            episode_number: row.episode_number,
            episode_name: row.episode_name,
            task_type: row.action.clone(),
            action: row.action,
            billing_meta: row.billing_meta.map(|value| value.0),
            created_at: row.created_at,
        })
        .collect::<Vec<_>>();

    Ok(TransactionListResult {
        transactions,
        total,
    })
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::decimal_quantity_to_i32;

    #[test]
    fn decimal_quantity_is_truncated_for_legacy_response_shape() {
        assert_eq!(decimal_quantity_to_i32(Decimal::new(59, 1)), 5);
    }
}
