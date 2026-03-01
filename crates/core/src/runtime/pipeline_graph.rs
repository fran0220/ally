use sqlx::MySqlPool;

use crate::errors::AppError;

use super::graph_executor::{GraphExecutorInput, GraphExecutorState, execute_pipeline_graph};

pub async fn run_pipeline_graph(
    pool: &MySqlPool,
    input: GraphExecutorInput,
) -> Result<GraphExecutorState, AppError> {
    execute_pipeline_graph(pool, input).await
}
