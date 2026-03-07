import { apiRequest } from './client';
import type { CapabilitySelections, ModelCapabilities } from '../lib/model-config-contract';

export type ModelType = 'llm' | 'image' | 'video' | 'audio' | 'lipsync';

export interface AdminProvider {
  id: string;
  name: string;
  baseUrl?: string;
  apiKey?: string;
  apiMode?: 'gemini-sdk';
}

export interface AdminModel {
  modelId: string;
  modelKey: string;
  name: string;
  type: ModelType;
  provider: string;
  enabled: boolean;
  price: number;
  capabilities?: ModelCapabilities;
  customPricing?: Record<string, unknown>;
}

export interface AdminDefaultModels {
  analysisModel?: string;
  characterModel?: string;
  locationModel?: string;
  storyboardModel?: string;
  editModel?: string;
  videoModel?: string;
  lipSyncModel?: string;
}

export interface AdminAiConfig {
  providers: AdminProvider[];
  models: AdminModel[];
  defaultModels?: AdminDefaultModels;
  capabilityDefaults?: CapabilitySelections;
}

export function getAdminAiConfig() {
  return apiRequest<AdminAiConfig>('/api/admin/ai-config');
}

export function updateAdminAiConfig(payload: AdminAiConfig) {
  return apiRequest<AdminAiConfig>('/api/admin/ai-config', {
    method: 'PUT',
    body: JSON.stringify(payload),
  });
}
