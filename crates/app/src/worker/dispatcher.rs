use deadpool_redis::Pool as RedisPool;
use sqlx::MySqlPool;
use tokio::time::{Duration, MissedTickBehavior, interval, sleep};
use tracing::{error, info, warn};
use waoowaoo_core::errors::AppError;

use crate::{consumer, handlers, task_context::TaskContext};

const CONSUME_ERROR_BACKOFF_MS: u64 = 500;
const TASK_HEARTBEAT_INTERVAL_SECS: u64 = 10;

async fn dispatch_with_heartbeat(
    task: &TaskContext,
    queue: &str,
) -> Result<handlers::DispatchResult, AppError> {
    let dispatch_future = handlers::dispatch(task);
    tokio::pin!(dispatch_future);

    let mut ticker = interval(Duration::from_secs(TASK_HEARTBEAT_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Consume the immediate first tick to ensure real 10s heartbeat cadence.
    ticker.tick().await;

    loop {
        tokio::select! {
            result = &mut dispatch_future => {
                return result;
            }
            _ = ticker.tick() => {
                if let Err(err) = task.refresh_heartbeat().await {
                    error!(
                        queue = queue,
                        task_id = task.task_id,
                        task_type = task.task_type,
                        error = %err,
                        "failed to refresh task heartbeat"
                    );
                }
            }
        }
    }
}

pub async fn run_worker_loop(
    queue: &str,
    billing_enabled: bool,
    mysql: MySqlPool,
    redis: RedisPool,
) -> Result<(), anyhow::Error> {
    info!(queue = queue, "worker loop started");

    loop {
        let task = match consumer::consume_next(&mysql, &redis, queue).await {
            Ok(task) => task,
            Err(err) => {
                error!(queue = queue, error = %err, "failed to consume task, retrying loop");
                sleep(Duration::from_millis(CONSUME_ERROR_BACKOFF_MS)).await;
                continue;
            }
        };
        let Some(task) = task else {
            continue;
        };
        let task = TaskContext::new(task, mysql.clone(), redis.clone(), queue, billing_enabled);

        match dispatch_with_heartbeat(&task, queue).await {
            Ok(dispatch_result) => match task.mark_completed(&dispatch_result).await {
                Ok(true) => {
                    info!(
                        task_id = task.task_id,
                        task_type = task.task_type,
                        "task handled"
                    );
                }
                Ok(false) => {
                    warn!(
                        queue = queue,
                        task_id = task.task_id,
                        task_type = task.task_type,
                        "skip mark_completed because task is no longer processing"
                    );
                }
                Err(err) => {
                    error!(
                        queue = queue,
                        task_id = task.task_id,
                        task_type = task.task_type,
                        error = %err,
                        "failed to persist completed task state"
                    );
                }
            },
            Err(err) => {
                if task.should_retry(&err) {
                    match task.requeue_for_retry().await {
                        Ok(true) => {
                            if let Err(publish_err) = task.publish_retry_scheduled(&err).await {
                                error!(
                                    queue = queue,
                                    task_id = task.task_id,
                                    task_type = task.task_type,
                                    error = %publish_err,
                                    "failed to publish retry scheduled event"
                                );
                            }

                            warn!(
                                queue = queue,
                                task_id = task.task_id,
                                task_type = task.task_type,
                                failed_attempt = task.failed_attempt(),
                                max_attempts = task.max_attempts(),
                                error = %err,
                                "task failed and was requeued for retry"
                            );
                        }
                        Ok(false) => {
                            warn!(
                                queue = queue,
                                task_id = task.task_id,
                                task_type = task.task_type,
                                "retry skipped because task is no longer processing"
                            );
                        }
                        Err(requeue_err) => {
                            error!(
                                queue = queue,
                                task_id = task.task_id,
                                task_type = task.task_type,
                                error = %requeue_err,
                                "failed to requeue task for retry"
                            );
                        }
                    }
                    continue;
                }

                match task.mark_failed(&err).await {
                    Ok(true) => {
                        error!(
                            task_id = task.task_id,
                            task_type = task.task_type,
                            error = %err,
                            "task handling failed"
                        );
                    }
                    Ok(false) => {
                        warn!(
                            queue = queue,
                            task_id = task.task_id,
                            task_type = task.task_type,
                            "skip mark_failed because task is no longer processing"
                        );
                    }
                    Err(db_err) => {
                        error!(
                            queue = queue,
                            task_id = task.task_id,
                            task_type = task.task_type,
                            error = %db_err,
                            "failed to persist failed task state"
                        );
                    }
                }
            }
        }
    }
}
