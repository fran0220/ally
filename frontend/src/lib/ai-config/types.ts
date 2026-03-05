import type { ModelCapabilities } from '../model-config-contract';

export type ModelType = 'llm' | 'image' | 'video' | 'audio' | 'lipsync';

export type ProviderApiMode = 'gemini-sdk' | 'openai-official';

export interface ParsedModelKey {
  provider: string;
  modelId: string;
}

export interface AiConfigProvider {
  id: string;
  name: string;
  baseUrl?: string;
  apiKey?: string;
  hasApiKey?: boolean;
  apiMode?: ProviderApiMode;
}

export interface AiConfigModel {
  modelId: string;
  modelKey: string;
  name: string;
  type: ModelType;
  provider: string;
  enabled: boolean;
  price: number;
  priceMin?: number;
  priceMax?: number;
  priceLabel?: string;
  priceInput?: number;
  priceOutput?: number;
  capabilities?: ModelCapabilities;
  customPricing?: Record<string, unknown>;
}

export interface AiConfigPayload {
  providers: AiConfigProvider[];
  models: AiConfigModel[];
}
