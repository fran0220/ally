pub mod image;
pub mod text;
pub mod video;
pub mod voice;

use serde_json::Value;
use waoowaoo_core::errors::AppError;

use crate::task_context::TaskContext;

pub type DispatchResult = Value;

pub async fn dispatch(task: &TaskContext) -> Result<DispatchResult, AppError> {
    match task.task_type.as_str() {
        "image_panel" => image::panel::handle(task).await,
        "image_character" => image::character::handle(task).await,
        "image_location" => image::location::handle(task).await,
        "panel_variant" => image::variant::handle(task).await,
        "modify_asset_image" => image::modify::handle(task).await,
        "regenerate_group" => image::regenerate_group::handle(task).await,
        "asset_hub_image" => image::asset_hub_image::handle(task).await,
        "asset_hub_modify" => image::asset_hub_modify::handle(task).await,

        "analyze_novel" => text::analyze_novel::handle(task).await,
        "analyze_global" => text::analyze_global::handle(task).await,
        "story_to_script_run" => text::story_to_script::handle(task).await,
        "script_to_storyboard_run" => text::script_to_storyboard::handle(task).await,
        "clips_build" => text::clips_build::handle(task).await,
        "screenplay_convert" => text::screenplay_convert::handle(task).await,
        "episode_split_llm" => text::episode_split::handle(task).await,
        "voice_analyze" => text::voice_analyze::handle(task).await,
        "ai_create_character" => text::ai_create_character::handle(task).await,
        "ai_create_location" => text::ai_create_location::handle(task).await,
        "ai_modify_appearance" => text::ai_modify_appearance::handle(task).await,
        "ai_modify_location" => text::ai_modify_location::handle(task).await,
        "ai_modify_shot_prompt" => text::ai_modify_shot_prompt::handle(task).await,
        "analyze_shot_variants" => text::analyze_shot_variants::handle(task).await,
        "character_profile_confirm" | "character_profile_batch_confirm" => {
            text::character_profile::handle(task).await
        }
        "reference_to_character" | "asset_hub_reference_to_character" => {
            text::reference_to_character::handle(task).await
        }
        "asset_hub_ai_design_character" | "asset_hub_ai_design_location" => {
            text::asset_hub_ai_design::handle(task).await
        }
        "asset_hub_ai_modify_character" | "asset_hub_ai_modify_location" => {
            text::asset_hub_ai_modify::handle(task).await
        }
        "regenerate_storyboard_text" => text::regenerate_text::handle(task).await,
        "insert_panel" => text::insert_panel::handle(task).await,

        "video_panel" => video::panel::handle(task).await,
        "lip_sync" => video::lip_sync::handle(task).await,

        "voice_line" => voice::voice_line::handle(task).await,
        "voice_design" => voice::voice_design::handle(task).await,
        "asset_hub_voice_design" => voice::asset_hub_voice_design::handle(task).await,

        _ => Err(AppError::invalid_params(format!(
            "unsupported task_type: {}",
            task.task_type
        ))),
    }
}
