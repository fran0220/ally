use serde_json::{Value, json};
use sqlx::FromRow;
use waoowaoo_core::errors::AppError;
use waoowaoo_core::prompt_i18n::{PromptIds, PromptVariables};

use crate::{runtime, task_context::TaskContext};

use super::shared;

#[derive(Debug, FromRow)]
struct EpisodeRow {
    id: String,
    #[sqlx(rename = "novelPromotionProjectId")]
    novel_promotion_project_id: String,
}

#[derive(Debug, FromRow)]
struct ClipRow {
    id: String,
    content: String,
}

#[derive(Debug, FromRow)]
struct CharacterIntroRow {
    name: String,
    introduction: Option<String>,
}

#[derive(Debug, FromRow)]
struct LocationRow {
    name: String,
}

pub async fn handle(task: &TaskContext) -> Result<Value, AppError> {
    shared::ensure_novel_project(task).await?;
    let mysql = runtime::mysql()?;
    let payload = &task.payload;
    let locale = shared::resolve_prompt_locale(payload);
    let list_separator = shared::l(locale, "、", ", ");
    let none_text = shared::l(locale, "无", "None");

    let episode_id = shared::read_episode_id(task)
        .ok_or_else(|| AppError::invalid_params("episodeId is required"))?;
    let novel_project = shared::get_novel_project(task).await?;
    let analysis_model = shared::resolve_analysis_model(task, payload).await?;

    let episode = sqlx::query_as::<_, EpisodeRow>(
        "SELECT id, novelPromotionProjectId FROM novel_promotion_episodes WHERE id = ? LIMIT 1",
    )
    .bind(&episode_id)
    .fetch_optional(mysql)
    .await?
    .ok_or_else(|| AppError::not_found("episode not found"))?;
    if episode.novel_promotion_project_id != novel_project.id {
        return Err(AppError::invalid_params(
            "episode does not belong to current project",
        ));
    }

    let clips = sqlx::query_as::<_, ClipRow>(
        "SELECT id, content FROM novel_promotion_clips WHERE episodeId = ? ORDER BY createdAt ASC",
    )
    .bind(&episode.id)
    .fetch_all(mysql)
    .await?;
    if clips.is_empty() {
        return Err(AppError::invalid_params(
            "screenplay_convert requires clips",
        ));
    }

    let character_rows = sqlx::query_as::<_, CharacterIntroRow>(
        "SELECT name, introduction FROM novel_promotion_characters WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;
    let location_rows = sqlx::query_as::<_, LocationRow>(
        "SELECT name FROM novel_promotion_locations WHERE novelPromotionProjectId = ? ORDER BY createdAt ASC",
    )
    .bind(&novel_project.id)
    .fetch_all(mysql)
    .await?;

    let characters_lib_name = character_rows
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(list_separator);
    let locations_lib_name = location_rows
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(list_separator);
    let characters_introduction = shared::build_characters_introduction(
        &character_rows
            .iter()
            .map(|item| (item.name.clone(), item.introduction.clone()))
            .collect::<Vec<_>>(),
        locale,
    );

    let _ = task
        .report_progress(10, Some("screenplay_convert_prepare"))
        .await?;

    let run_id = shared::create_stream_run_id(task, "screenplay_convert");
    let total = clips.len();
    let mut total_scenes = 0usize;
    let mut results = Vec::with_capacity(total);

    for (index, clip) in clips.iter().enumerate() {
        let clip_content = clip.content.trim();
        if clip_content.is_empty() {
            return Err(AppError::invalid_params(format!(
                "clip {} content is empty",
                clip.id
            )));
        }

        let progress = 15 + (((index + 1) as i32 * 70) / i32::try_from(total).unwrap_or(1));
        let _ = task
            .report_progress(progress, Some("screenplay_convert_step"))
            .await?;

        let mut prompt_variables = PromptVariables::new();
        prompt_variables.insert("clip_content".to_string(), clip_content.to_string());
        prompt_variables.insert(
            "locations_lib_name".to_string(),
            if locations_lib_name.is_empty() {
                none_text.to_string()
            } else {
                locations_lib_name.clone()
            },
        );
        prompt_variables.insert(
            "characters_lib_name".to_string(),
            if characters_lib_name.is_empty() {
                none_text.to_string()
            } else {
                characters_lib_name.clone()
            },
        );
        prompt_variables.insert(
            "characters_introduction".to_string(),
            characters_introduction.clone(),
        );
        prompt_variables.insert("clip_id".to_string(), clip.id.clone());

        let prompt = shared::render_prompt_template(
            payload,
            PromptIds::NP_SCREENPLAY_CONVERSION,
            &prompt_variables,
        )?;

        let step_meta = shared::LlmStepMeta {
            run_id: Some(run_id.clone()),
            stream_run_id: Some(run_id.clone()),
            step_id: Some(format!("screenplay_clip_{}", clip.id)),
            step_title: Some(format!(
                "{} {}/{}",
                shared::l(locale, "片段剧本转换", "Screenplay Conversion"),
                index + 1,
                total
            )),
            step_attempt: Some(1),
            step_index: Some(i32::try_from(index + 1).unwrap_or(1)),
            step_total: Some(i32::try_from(total).unwrap_or(1)),
        };

        let response = shared::chat_with_step(task, &analysis_model, &prompt, &step_meta).await?;
        let mut screenplay = shared::parse_json_object_response(&response)?;
        if let Some(object) = screenplay.as_object_mut() {
            object.insert("clip_id".to_string(), Value::String(clip.id.clone()));
            object.insert(
                "original_text".to_string(),
                Value::String(clip_content.to_string()),
            );
        }

        let scene_count = screenplay
            .get("scenes")
            .and_then(Value::as_array)
            .map(|items| items.len())
            .unwrap_or(0);
        total_scenes += scene_count;

        sqlx::query(
            "UPDATE novel_promotion_clips SET screenplay = ?, updatedAt = NOW(3) WHERE id = ?",
        )
        .bind(serde_json::to_string(&screenplay).map_err(|err| {
            AppError::internal(format!("failed to encode screenplay json: {err}"))
        })?)
        .bind(&clip.id)
        .execute(mysql)
        .await?;

        results.push(json!({
            "clipId": clip.id,
            "success": true,
            "sceneCount": scene_count,
        }));
    }

    let _ = task
        .report_progress(96, Some("screenplay_convert_done"))
        .await?;

    Ok(json!({
        "episodeId": episode.id,
        "total": total,
        "successCount": total,
        "failCount": 0,
        "totalScenes": total_scenes,
        "results": results,
    }))
}
