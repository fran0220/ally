use serde_json::Value;
use waoowaoo_core::errors::AppError;

use crate::task_context::TaskContext;

use super::asset_hub_ai_design;

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    asset_hub_ai_design::handle_for_asset_type(task, "location").await
}
