use serde_json::Value;
use waoowaoo_core::errors::AppError;

use crate::task_context::TaskContext;

use super::analyze_novel;

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    analyze_novel::handle(task).await
}
