use super::{PromptId, types::PromptCatalogEntry};

const EMPTY: &[&str] = &[];
const KEYS_AGENT_ACTING_DIRECTION: &[&str] = &["panels_json", "panel_count", "characters_info"];
const KEYS_AGENT_CHARACTER_PROFILE: &[&str] = &["input", "characters_lib_info"];
const KEYS_AGENT_CHARACTER_VISUAL: &[&str] = &["character_profiles"];
const KEYS_AGENT_CINEMATOGRAPHER: &[&str] = &[
    "panels_json",
    "panel_count",
    "locations_description",
    "characters_info",
];
const KEYS_AGENT_CLIP: &[&str] = &[
    "input",
    "locations_lib_name",
    "characters_lib_name",
    "characters_introduction",
];
const KEYS_VARIANT_ANALYSIS: &[&str] = &[
    "panel_description",
    "shot_type",
    "camera_move",
    "location",
    "characters_info",
];
const KEYS_VARIANT_GENERATE: &[&str] = &[
    "original_description",
    "original_shot_type",
    "original_camera_move",
    "location",
    "characters_info",
    "variant_title",
    "variant_description",
    "target_shot_type",
    "target_camera_move",
    "video_prompt",
    "character_assets",
    "location_asset",
    "aspect_ratio",
    "style",
];
const KEYS_STORYBOARD_DETAIL: &[&str] = &[
    "panels_json",
    "characters_age_gender",
    "locations_description",
];
const KEYS_STORYBOARD_INSERT: &[&str] = &[
    "prev_panel_json",
    "next_panel_json",
    "characters_full_description",
    "locations_description",
    "user_input",
];
const KEYS_STORYBOARD_PLAN: &[&str] = &[
    "characters_lib_name",
    "locations_lib_name",
    "characters_introduction",
    "characters_appearance_list",
    "characters_full_description",
    "clip_json",
    "clip_content",
];
const KEYS_CHARACTER_CREATE: &[&str] = &["user_input"];
const KEYS_CHARACTER_DESCRIPTION_UPDATE: &[&str] = &[
    "original_description",
    "modify_instruction",
    "image_context",
];
const KEYS_CHARACTER_MODIFY: &[&str] = &["character_input", "user_input"];
const KEYS_CHARACTER_REGENERATE: &[&str] = &[
    "character_name",
    "current_descriptions",
    "change_reason",
    "novel_text",
];
const KEYS_EPISODE_SPLIT: &[&str] = &["CONTENT"];
const KEYS_IMAGE_PROMPT_MODIFY: &[&str] = &["prompt_input", "user_input", "video_prompt_input"];
const KEYS_LOCATION_CREATE: &[&str] = &["user_input"];
const KEYS_LOCATION_DESCRIPTION_UPDATE: &[&str] = &[
    "location_name",
    "original_description",
    "modify_instruction",
    "image_context",
];
const KEYS_LOCATION_MODIFY: &[&str] = &["location_name", "location_input", "user_input"];
const KEYS_LOCATION_REGENERATE: &[&str] = &["location_name", "current_descriptions"];
const KEYS_SCREENPLAY_CONVERSION: &[&str] = &[
    "clip_content",
    "locations_lib_name",
    "characters_lib_name",
    "characters_introduction",
    "clip_id",
];
const KEYS_SELECT_LOCATION: &[&str] = &["input", "locations_lib_name"];
const KEYS_SINGLE_PANEL_IMAGE: &[&str] = &[
    "storyboard_text_json_input",
    "source_text",
    "aspect_ratio",
    "style",
];
const KEYS_STORYBOARD_EDIT: &[&str] = &["user_input"];
const KEYS_VOICE_ANALYSIS: &[&str] = &[
    "input",
    "characters_lib_name",
    "characters_introduction",
    "storyboard_json",
];

pub const fn prompt_catalog_entry(prompt_id: PromptId) -> PromptCatalogEntry {
    match prompt_id {
        PromptId::CharacterImageToDescription => PromptCatalogEntry {
            path_stem: "character-reference/character_image_to_description",
            variable_keys: EMPTY,
        },
        PromptId::CharacterReferenceToSheet => PromptCatalogEntry {
            path_stem: "character-reference/character_reference_to_sheet",
            variable_keys: EMPTY,
        },
        PromptId::NpAgentActingDirection => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_acting_direction",
            variable_keys: KEYS_AGENT_ACTING_DIRECTION,
        },
        PromptId::NpAgentCharacterProfile => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_character_profile",
            variable_keys: KEYS_AGENT_CHARACTER_PROFILE,
        },
        PromptId::NpAgentCharacterVisual => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_character_visual",
            variable_keys: KEYS_AGENT_CHARACTER_VISUAL,
        },
        PromptId::NpAgentCinematographer => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_cinematographer",
            variable_keys: KEYS_AGENT_CINEMATOGRAPHER,
        },
        PromptId::NpAgentClip => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_clip",
            variable_keys: KEYS_AGENT_CLIP,
        },
        PromptId::NpAgentShotVariantAnalysis => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_shot_variant_analysis",
            variable_keys: KEYS_VARIANT_ANALYSIS,
        },
        PromptId::NpAgentShotVariantGenerate => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_shot_variant_generate",
            variable_keys: KEYS_VARIANT_GENERATE,
        },
        PromptId::NpAgentStoryboardDetail => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_storyboard_detail",
            variable_keys: KEYS_STORYBOARD_DETAIL,
        },
        PromptId::NpAgentStoryboardInsert => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_storyboard_insert",
            variable_keys: KEYS_STORYBOARD_INSERT,
        },
        PromptId::NpAgentStoryboardPlan => PromptCatalogEntry {
            path_stem: "novel-promotion/agent_storyboard_plan",
            variable_keys: KEYS_STORYBOARD_PLAN,
        },
        PromptId::NpCharacterCreate => PromptCatalogEntry {
            path_stem: "novel-promotion/character_create",
            variable_keys: KEYS_CHARACTER_CREATE,
        },
        PromptId::NpCharacterDescriptionUpdate => PromptCatalogEntry {
            path_stem: "novel-promotion/character_description_update",
            variable_keys: KEYS_CHARACTER_DESCRIPTION_UPDATE,
        },
        PromptId::NpCharacterModify => PromptCatalogEntry {
            path_stem: "novel-promotion/character_modify",
            variable_keys: KEYS_CHARACTER_MODIFY,
        },
        PromptId::NpCharacterRegenerate => PromptCatalogEntry {
            path_stem: "novel-promotion/character_regenerate",
            variable_keys: KEYS_CHARACTER_REGENERATE,
        },
        PromptId::NpEpisodeSplit => PromptCatalogEntry {
            path_stem: "novel-promotion/episode_split",
            variable_keys: KEYS_EPISODE_SPLIT,
        },
        PromptId::NpImagePromptModify => PromptCatalogEntry {
            path_stem: "novel-promotion/image_prompt_modify",
            variable_keys: KEYS_IMAGE_PROMPT_MODIFY,
        },
        PromptId::NpLocationCreate => PromptCatalogEntry {
            path_stem: "novel-promotion/location_create",
            variable_keys: KEYS_LOCATION_CREATE,
        },
        PromptId::NpLocationDescriptionUpdate => PromptCatalogEntry {
            path_stem: "novel-promotion/location_description_update",
            variable_keys: KEYS_LOCATION_DESCRIPTION_UPDATE,
        },
        PromptId::NpLocationModify => PromptCatalogEntry {
            path_stem: "novel-promotion/location_modify",
            variable_keys: KEYS_LOCATION_MODIFY,
        },
        PromptId::NpLocationRegenerate => PromptCatalogEntry {
            path_stem: "novel-promotion/location_regenerate",
            variable_keys: KEYS_LOCATION_REGENERATE,
        },
        PromptId::NpScreenplayConversion => PromptCatalogEntry {
            path_stem: "novel-promotion/screenplay_conversion",
            variable_keys: KEYS_SCREENPLAY_CONVERSION,
        },
        PromptId::NpSelectLocation => PromptCatalogEntry {
            path_stem: "novel-promotion/select_location",
            variable_keys: KEYS_SELECT_LOCATION,
        },
        PromptId::NpSinglePanelImage => PromptCatalogEntry {
            path_stem: "novel-promotion/single_panel_image",
            variable_keys: KEYS_SINGLE_PANEL_IMAGE,
        },
        PromptId::NpStoryboardEdit => PromptCatalogEntry {
            path_stem: "novel-promotion/storyboard_edit",
            variable_keys: KEYS_STORYBOARD_EDIT,
        },
        PromptId::NpVoiceAnalysis => PromptCatalogEntry {
            path_stem: "novel-promotion/voice_analysis",
            variable_keys: KEYS_VOICE_ANALYSIS,
        },
    }
}
