use serde_json::Value;
use waoowaoo_core::errors::AppError;

use crate::task_context::TaskContext;

use super::voice_design;

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    voice_design::handle_with_options(task).await
}
