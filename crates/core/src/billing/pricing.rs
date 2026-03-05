use sqlx::MySqlPool;

use crate::{api_config::parse_model_key_strict, errors::AppError};

use super::types::ModelPrice;

fn model_id_candidates(raw_model: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let normalized = raw_model.trim();

    if normalized.is_empty() {
        return candidates;
    }

    candidates.push(normalized.to_string());

    if let Some(parsed) = parse_model_key_strict(normalized)
        && !parsed.model_id.is_empty()
    {
        candidates.push(parsed.model_id);
    }

    if let Some(last_segment) = normalized.rsplit("::").next() {
        let trimmed = last_segment.trim();
        if !trimmed.is_empty() {
            candidates.push(trimmed.to_string());
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

pub async fn get_unit_price(
    pool: &MySqlPool,
    api_type: &str,
    model_id: &str,
    unit: &str,
) -> Result<ModelPrice, AppError> {
    let api_type = api_type.trim();
    let unit = unit.trim();
    if api_type.is_empty() || unit.is_empty() {
        return Err(AppError::invalid_params(
            "api_type and unit are required for pricing lookup",
        ));
    }

    let candidates = model_id_candidates(model_id);
    if candidates.is_empty() {
        return Err(AppError::invalid_params(
            "model id is required for pricing lookup",
        ));
    }

    for candidate in candidates {
        let row = sqlx::query_as::<_, ModelPrice>(
            "SELECT api_type, model_id, unit, unit_price FROM model_pricing WHERE api_type = ? AND model_id = ? AND unit = ? LIMIT 1",
        )
        .bind(api_type)
        .bind(&candidate)
        .bind(unit)
        .fetch_optional(pool)
        .await?;

        if let Some(price) = row {
            return Ok(price);
        }
    }

    Err(AppError::not_found(format!(
        "billing pricing not found for api_type={api_type}, model={model_id}, unit={unit}",
    )))
}
