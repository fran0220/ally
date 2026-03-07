use serde_json::{Value, json};
use waoowaoo_core::errors::AppError;

use crate::{consumer::WorkerTask, task_context::TaskContext};

use super::{analyze_novel, clips_build, screenplay_convert, shared};

fn with_episode_payload(task: &WorkerTask) -> WorkerTask {
    let mut next = task.clone();
    if shared::read_string(&next.payload, "episodeId").is_some() {
        return next;
    }

    let Some(episode_id) = task.episode_id.clone() else {
        return next;
    };

    if let Some(object) = next.payload.as_object_mut() {
        object.insert("episodeId".to_string(), Value::String(episode_id));
    } else {
        next.payload = json!({ "episodeId": episode_id });
    }
    next
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    let task_with_episode = task.with_task(with_episode_payload(task));

    let _ = task
        .report_progress(10, Some("progress.stage.storyToScriptPrepare"))
        .await?;
    let analyze_result = analyze_novel::handle(task).await?;

    let _ = task
        .report_progress(45, Some("progress.stage.storyToScriptClips"))
        .await?;
    let clips_result = clips_build::handle(&task_with_episode).await?;

    let _ = task
        .report_progress(70, Some("progress.stage.storyToScriptScreenplay"))
        .await?;
    let screenplay_result = screenplay_convert::handle(&task_with_episode).await?;

    let _ = task
        .report_progress(96, Some("progress.stage.storyToScriptDone"))
        .await?;

    Ok(json!({
        "success": true,
        "pipeline": ["analyze_novel", "clips_build", "screenplay_convert"],
        "analyze": analyze_result,
        "clips": clips_result,
        "screenplay": screenplay_result,
    }))
}
