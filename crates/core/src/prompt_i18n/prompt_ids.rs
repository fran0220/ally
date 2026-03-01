use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptId {
    CharacterImageToDescription,
    CharacterReferenceToSheet,
    NpAgentActingDirection,
    NpAgentCharacterProfile,
    NpAgentCharacterVisual,
    NpAgentCinematographer,
    NpAgentClip,
    NpAgentShotVariantAnalysis,
    NpAgentShotVariantGenerate,
    NpAgentStoryboardDetail,
    NpAgentStoryboardInsert,
    NpAgentStoryboardPlan,
    NpCharacterCreate,
    NpCharacterDescriptionUpdate,
    NpCharacterModify,
    NpCharacterRegenerate,
    NpEpisodeSplit,
    NpImagePromptModify,
    NpLocationCreate,
    NpLocationDescriptionUpdate,
    NpLocationModify,
    NpLocationRegenerate,
    NpScreenplayConversion,
    NpSelectLocation,
    NpSinglePanelImage,
    NpStoryboardEdit,
    NpVoiceAnalysis,
}

impl PromptId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CharacterImageToDescription => "character_image_to_description",
            Self::CharacterReferenceToSheet => "character_reference_to_sheet",
            Self::NpAgentActingDirection => "np_agent_acting_direction",
            Self::NpAgentCharacterProfile => "np_agent_character_profile",
            Self::NpAgentCharacterVisual => "np_agent_character_visual",
            Self::NpAgentCinematographer => "np_agent_cinematographer",
            Self::NpAgentClip => "np_agent_clip",
            Self::NpAgentShotVariantAnalysis => "np_agent_shot_variant_analysis",
            Self::NpAgentShotVariantGenerate => "np_agent_shot_variant_generate",
            Self::NpAgentStoryboardDetail => "np_agent_storyboard_detail",
            Self::NpAgentStoryboardInsert => "np_agent_storyboard_insert",
            Self::NpAgentStoryboardPlan => "np_agent_storyboard_plan",
            Self::NpCharacterCreate => "np_character_create",
            Self::NpCharacterDescriptionUpdate => "np_character_description_update",
            Self::NpCharacterModify => "np_character_modify",
            Self::NpCharacterRegenerate => "np_character_regenerate",
            Self::NpEpisodeSplit => "np_episode_split",
            Self::NpImagePromptModify => "np_image_prompt_modify",
            Self::NpLocationCreate => "np_location_create",
            Self::NpLocationDescriptionUpdate => "np_location_description_update",
            Self::NpLocationModify => "np_location_modify",
            Self::NpLocationRegenerate => "np_location_regenerate",
            Self::NpScreenplayConversion => "np_screenplay_conversion",
            Self::NpSelectLocation => "np_select_location",
            Self::NpSinglePanelImage => "np_single_panel_image",
            Self::NpStoryboardEdit => "np_storyboard_edit",
            Self::NpVoiceAnalysis => "np_voice_analysis",
        }
    }
}

impl FromStr for PromptId {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim() {
            "character_image_to_description" => Ok(Self::CharacterImageToDescription),
            "character_reference_to_sheet" => Ok(Self::CharacterReferenceToSheet),
            "np_agent_acting_direction" => Ok(Self::NpAgentActingDirection),
            "np_agent_character_profile" => Ok(Self::NpAgentCharacterProfile),
            "np_agent_character_visual" => Ok(Self::NpAgentCharacterVisual),
            "np_agent_cinematographer" => Ok(Self::NpAgentCinematographer),
            "np_agent_clip" => Ok(Self::NpAgentClip),
            "np_agent_shot_variant_analysis" => Ok(Self::NpAgentShotVariantAnalysis),
            "np_agent_shot_variant_generate" => Ok(Self::NpAgentShotVariantGenerate),
            "np_agent_storyboard_detail" => Ok(Self::NpAgentStoryboardDetail),
            "np_agent_storyboard_insert" => Ok(Self::NpAgentStoryboardInsert),
            "np_agent_storyboard_plan" => Ok(Self::NpAgentStoryboardPlan),
            "np_character_create" => Ok(Self::NpCharacterCreate),
            "np_character_description_update" => Ok(Self::NpCharacterDescriptionUpdate),
            "np_character_modify" => Ok(Self::NpCharacterModify),
            "np_character_regenerate" => Ok(Self::NpCharacterRegenerate),
            "np_episode_split" => Ok(Self::NpEpisodeSplit),
            "np_image_prompt_modify" => Ok(Self::NpImagePromptModify),
            "np_location_create" => Ok(Self::NpLocationCreate),
            "np_location_description_update" => Ok(Self::NpLocationDescriptionUpdate),
            "np_location_modify" => Ok(Self::NpLocationModify),
            "np_location_regenerate" => Ok(Self::NpLocationRegenerate),
            "np_screenplay_conversion" => Ok(Self::NpScreenplayConversion),
            "np_select_location" => Ok(Self::NpSelectLocation),
            "np_single_panel_image" => Ok(Self::NpSinglePanelImage),
            "np_storyboard_edit" => Ok(Self::NpStoryboardEdit),
            "np_voice_analysis" => Ok(Self::NpVoiceAnalysis),
            _ => Err(format!("unknown prompt id: {raw}")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PromptIds;

impl PromptIds {
    pub const CHARACTER_IMAGE_TO_DESCRIPTION: PromptId = PromptId::CharacterImageToDescription;
    pub const CHARACTER_REFERENCE_TO_SHEET: PromptId = PromptId::CharacterReferenceToSheet;
    pub const NP_AGENT_ACTING_DIRECTION: PromptId = PromptId::NpAgentActingDirection;
    pub const NP_AGENT_CHARACTER_PROFILE: PromptId = PromptId::NpAgentCharacterProfile;
    pub const NP_AGENT_CHARACTER_VISUAL: PromptId = PromptId::NpAgentCharacterVisual;
    pub const NP_AGENT_CINEMATOGRAPHER: PromptId = PromptId::NpAgentCinematographer;
    pub const NP_AGENT_CLIP: PromptId = PromptId::NpAgentClip;
    pub const NP_AGENT_SHOT_VARIANT_ANALYSIS: PromptId = PromptId::NpAgentShotVariantAnalysis;
    pub const NP_AGENT_SHOT_VARIANT_GENERATE: PromptId = PromptId::NpAgentShotVariantGenerate;
    pub const NP_AGENT_STORYBOARD_DETAIL: PromptId = PromptId::NpAgentStoryboardDetail;
    pub const NP_AGENT_STORYBOARD_INSERT: PromptId = PromptId::NpAgentStoryboardInsert;
    pub const NP_AGENT_STORYBOARD_PLAN: PromptId = PromptId::NpAgentStoryboardPlan;
    pub const NP_CHARACTER_CREATE: PromptId = PromptId::NpCharacterCreate;
    pub const NP_CHARACTER_DESCRIPTION_UPDATE: PromptId = PromptId::NpCharacterDescriptionUpdate;
    pub const NP_CHARACTER_MODIFY: PromptId = PromptId::NpCharacterModify;
    pub const NP_CHARACTER_REGENERATE: PromptId = PromptId::NpCharacterRegenerate;
    pub const NP_EPISODE_SPLIT: PromptId = PromptId::NpEpisodeSplit;
    pub const NP_IMAGE_PROMPT_MODIFY: PromptId = PromptId::NpImagePromptModify;
    pub const NP_LOCATION_CREATE: PromptId = PromptId::NpLocationCreate;
    pub const NP_LOCATION_DESCRIPTION_UPDATE: PromptId = PromptId::NpLocationDescriptionUpdate;
    pub const NP_LOCATION_MODIFY: PromptId = PromptId::NpLocationModify;
    pub const NP_LOCATION_REGENERATE: PromptId = PromptId::NpLocationRegenerate;
    pub const NP_SCREENPLAY_CONVERSION: PromptId = PromptId::NpScreenplayConversion;
    pub const NP_SELECT_LOCATION: PromptId = PromptId::NpSelectLocation;
    pub const NP_SINGLE_PANEL_IMAGE: PromptId = PromptId::NpSinglePanelImage;
    pub const NP_STORYBOARD_EDIT: PromptId = PromptId::NpStoryboardEdit;
    pub const NP_VOICE_ANALYSIS: PromptId = PromptId::NpVoiceAnalysis;
}

pub const PROMPT_IDS: PromptIds = PromptIds;
