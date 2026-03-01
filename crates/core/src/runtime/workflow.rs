use std::collections::HashSet;

use once_cell::sync::Lazy;

static AI_TASK_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "image_panel",
        "image_character",
        "image_location",
        "panel_variant",
        "modify_asset_image",
        "regenerate_group",
        "asset_hub_image",
        "asset_hub_modify",
        "analyze_novel",
        "analyze_global",
        "story_to_script_run",
        "script_to_storyboard_run",
        "clips_build",
        "screenplay_convert",
        "episode_split_llm",
        "voice_analyze",
        "ai_create_character",
        "ai_create_location",
        "ai_modify_appearance",
        "ai_modify_location",
        "ai_modify_shot_prompt",
        "analyze_shot_variants",
        "character_profile_confirm",
        "character_profile_batch_confirm",
        "reference_to_character",
        "asset_hub_reference_to_character",
        "asset_hub_ai_design_character",
        "asset_hub_ai_design_location",
        "asset_hub_ai_modify_character",
        "asset_hub_ai_modify_location",
        "regenerate_storyboard_text",
        "insert_panel",
        "video_panel",
        "lip_sync",
        "voice_line",
        "voice_design",
        "asset_hub_voice_design",
    ])
});

pub fn is_ai_task_type(task_type: &str) -> bool {
    AI_TASK_TYPES.contains(task_type.trim())
}

pub fn workflow_type_from_task_type(task_type: &str) -> String {
    task_type.trim().to_string()
}
