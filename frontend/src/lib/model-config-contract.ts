export type CapabilityValue = string | number | boolean;

export type CapabilitySelections = Record<string, Record<string, CapabilityValue>>;

export interface CapabilityFieldI18n {
  labelKey?: string;
  unitKey?: string;
  optionLabelKeys?: Record<string, string>;
}

export interface LLMCapabilities {
  reasoningEffortOptions?: string[];
}

export interface ImageCapabilities {
  resolutionOptions?: string[];
}

export interface VideoCapabilities {
  generationModeOptions?: string[];
  generateAudioOptions?: boolean[];
  durationOptions?: number[];
  fpsOptions?: number[];
  resolutionOptions?: string[];
  fieldI18n?: Record<string, CapabilityFieldI18n>;
  firstlastframe?: boolean;
  supportGenerateAudio?: boolean;
}

export interface AudioCapabilities {
  voiceOptions?: string[];
  rateOptions?: string[];
}

export interface LipSyncCapabilities {
  modeOptions?: string[];
}

export interface ModelCapabilities {
  llm?: LLMCapabilities;
  image?: ImageCapabilities;
  video?: VideoCapabilities;
  audio?: AudioCapabilities;
  lipsync?: LipSyncCapabilities;
}
