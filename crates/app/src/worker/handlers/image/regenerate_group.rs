use serde_json::Value;
use waoowaoo_core::errors::AppError;

use crate::task_context::TaskContext;

use super::{character, location, shared};

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let regenerate_type = shared::read_string(&task.payload, "type")
        .unwrap_or_else(|| "location".to_string())
        .to_lowercase();

    if regenerate_type == "character" {
        character::handle(task).await
    } else {
        location::handle(task).await
    }
}
