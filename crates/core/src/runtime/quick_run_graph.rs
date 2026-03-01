use sqlx::MySqlPool;

use crate::errors::AppError;

use super::{
    graph_executor::{GraphExecutorInput, GraphExecutorState, GraphNode},
    pipeline_graph::run_pipeline_graph,
};

pub struct QuickRunGraphInput {
    pub run_id: String,
    pub project_id: String,
    pub user_id: String,
    pub node: GraphNode,
    pub state: GraphExecutorState,
}

pub async fn run_quick_run_graph(
    pool: &MySqlPool,
    input: QuickRunGraphInput,
) -> Result<GraphExecutorState, AppError> {
    run_pipeline_graph(
        pool,
        GraphExecutorInput {
            run_id: input.run_id,
            project_id: input.project_id,
            user_id: input.user_id,
            state: input.state,
            nodes: vec![input.node],
        },
    )
    .await
}
