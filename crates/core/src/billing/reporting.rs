use chrono::NaiveDateTime;
use rust_decimal::Decimal;
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
    #[sqlx(rename = "projectId")]
    project_id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "apiType")]
    api_type: String,
    model: String,
    action: String,
    quantity: i32,
    unit: String,
    cost: Decimal,
    metadata: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct TotalRow {
    total: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct TransactionRow {
    id: String,
    #[sqlx(rename = "userId")]
    user_id: String,
    #[sqlx(rename = "type")]
    tx_type: String,
    amount: Decimal,
    #[sqlx(rename = "balanceAfter")]
    balance_after: Decimal,
    description: Option<String>,
    #[sqlx(rename = "relatedId")]
    related_id: Option<String>,
    #[sqlx(rename = "freezeId")]
    freeze_id: Option<String>,
    #[sqlx(rename = "operatorId")]
    operator_id: Option<String>,
    #[sqlx(rename = "externalOrderId")]
    external_order_id: Option<String>,
    #[sqlx(rename = "idempotencyKey")]
    idempotency_key: Option<String>,
    #[sqlx(rename = "projectId")]
    project_id: Option<String>,
    #[sqlx(rename = "projectName")]
    project_name: Option<String>,
    #[sqlx(rename = "episodeId")]
    episode_id: Option<String>,
    #[sqlx(rename = "episodeNumber")]
    episode_number: Option<i32>,
    #[sqlx(rename = "episodeName")]
    episode_name: Option<String>,
    #[sqlx(rename = "taskType")]
    task_type: Option<String>,
    #[sqlx(rename = "billingMeta")]
    billing_meta: Option<String>,
    #[sqlx(rename = "createdAt")]
    created_at: NaiveDateTime,
}

fn parse_action_from_description(description: Option<&str>) -> Option<String> {
    let description = description?.trim();
    if description.is_empty() {
        return None;
    }

    let cleaned = description
        .strip_prefix("[SHADOW]")
        .map(str::trim)
        .unwrap_or(description);
    let action = cleaned.split(" - ").next()?.trim();
    if action.is_empty() {
        return None;
    }

    if action
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        && action
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        return Some(action.to_string());
    }

    None
}

pub async fn get_project_total_cost(pool: &MySqlPool, project_id: &str) -> Result<f64, AppError> {
    let total = sqlx::query_as::<_, TotalRow>(
        "SELECT COALESCE(SUM(cost), 0) AS total FROM usage_costs WHERE projectId = ?",
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
        "SELECT apiType AS `key`, COALESCE(SUM(cost), 0) AS total_cost, COUNT(*) AS record_count FROM usage_costs WHERE projectId = ? GROUP BY apiType",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let by_action_raw = sqlx::query_as::<_, CostAggregateRow>(
        "SELECT action AS `key`, COALESCE(SUM(cost), 0) AS total_cost, COUNT(*) AS record_count FROM usage_costs WHERE projectId = ? GROUP BY action",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let recent = sqlx::query_as::<_, UsageCostRow>(
        "SELECT id, projectId, userId, apiType, model, action, quantity, unit, cost, metadata, createdAt FROM usage_costs WHERE projectId = ? ORDER BY createdAt DESC LIMIT 50",
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
            quantity: row.quantity,
            unit: row.unit,
            cost: decimal_to_f64(row.cost),
            metadata: row.metadata,
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
        "SELECT projectId AS `key`, COALESCE(SUM(cost), 0) AS total_cost, COUNT(*) AS record_count FROM usage_costs WHERE userId = ? GROUP BY projectId",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_as::<_, TotalRow>(
        "SELECT COALESCE(SUM(cost), 0) AS total FROM usage_costs WHERE userId = ?",
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
        "SELECT id, projectId, userId, apiType, model, action, quantity, unit, cost, metadata, createdAt FROM usage_costs WHERE userId = ? ORDER BY createdAt DESC LIMIT ? OFFSET ?",
    )
    .bind(user_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM usage_costs WHERE userId = ?")
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
                quantity: row.quantity,
                unit: row.unit,
                cost: decimal_to_f64(row.cost),
                metadata: row.metadata,
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
        qb.push(" AND bt.type = ");
        qb.push_bind(tx_type);
    }

    if let Some(start_at) = input.start_at {
        qb.push(" AND bt.createdAt >= ");
        qb.push_bind(start_at);
    }
    if let Some(end_at) = input.end_at {
        qb.push(" AND bt.createdAt <= ");
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

    let mut count_qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT COUNT(*) AS total FROM balance_transactions bt WHERE bt.userId = ",
    );
    count_qb.push_bind(&input.user_id);
    push_transaction_filters(&mut count_qb, input);

    let total = count_qb.build_query_scalar::<i64>().fetch_one(pool).await?;

    let mut data_qb: QueryBuilder<'_, MySql> = QueryBuilder::new(
        "SELECT bt.id, bt.userId, bt.type, bt.amount, bt.balanceAfter, bt.description, bt.relatedId, bt.freezeId, bt.operatorId, bt.externalOrderId, bt.idempotencyKey, bt.projectId, p.name AS projectName, bt.episodeId, e.episodeNumber AS episodeNumber, e.name AS episodeName, bt.taskType, bt.billingMeta, bt.createdAt FROM balance_transactions bt LEFT JOIN projects p ON p.id = bt.projectId LEFT JOIN novel_promotion_episodes e ON e.id = bt.episodeId WHERE bt.userId = ",
    );
    data_qb.push_bind(&input.user_id);
    push_transaction_filters(&mut data_qb, input);
    data_qb.push(" ORDER BY bt.createdAt DESC LIMIT ");
    data_qb.push_bind(page_size);
    data_qb.push(" OFFSET ");
    data_qb.push_bind(offset);

    let rows = data_qb
        .build_query_as::<TransactionRow>()
        .fetch_all(pool)
        .await?;

    let transactions = rows
        .into_iter()
        .map(|row| {
            let billing_meta = row
                .billing_meta
                .as_deref()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok());

            UserTransactionRecord {
                id: row.id,
                user_id: row.user_id,
                tx_type: row.tx_type,
                amount: decimal_to_f64(row.amount),
                balance_after: decimal_to_f64(row.balance_after),
                description: row.description.clone(),
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
                action: row
                    .task_type
                    .as_ref()
                    .and_then(|value| {
                        let value = value.trim();
                        if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    })
                    .or_else(|| parse_action_from_description(row.description.as_deref())),
                task_type: row.task_type,
                billing_meta,
                created_at: row.created_at,
            }
        })
        .collect::<Vec<_>>();

    Ok(TransactionListResult {
        transactions,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_action_from_description;

    #[test]
    fn parse_action_from_description_handles_shadow_prefix() {
        assert_eq!(
            parse_action_from_description(Some("[SHADOW] modify_asset_image - model-a - ¥0.96")),
            Some("modify_asset_image".to_string())
        );
    }

    #[test]
    fn parse_action_from_description_ignores_non_action_format() {
        assert_eq!(
            parse_action_from_description(Some("Balance recharge | audit={...}")),
            None
        );
    }
}
