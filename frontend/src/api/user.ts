import { apiRequest } from './client';
import type { CapabilitySelections, ModelCapabilities } from '../lib/model-config-contract';

export interface UserPreference {
  id: string;
  userId: string;
  analysisModel: string | null;
  characterModel: string | null;
  locationModel: string | null;
  storyboardModel: string | null;
  editModel: string | null;
  videoModel: string | null;
  lipSyncModel: string | null;
  videoRatio: string;
  artStyle: string;
  ttsRate: string;
  createdAt: string;
  updatedAt: string;
}

export function getPreference() {
  return apiRequest<{ preference: UserPreference }>('/api/user-preference');
}

export function updatePreference(payload: Partial<UserPreference>) {
  return apiRequest<{ preference: UserPreference }>('/api/user-preference', {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export interface UserModelOption {
  value: string;
  label: string;
  provider: string;
  providerName?: string;
}

export interface UserModelsPayload {
  llm: UserModelOption[];
  image: UserModelOption[];
  video: UserModelOption[];
  audio: UserModelOption[];
  lipsync: UserModelOption[];
}

export function listUserModels() {
  return apiRequest<UserModelsPayload>('/api/user/models');
}

export type UserApiMode = 'gemini-sdk' | 'openai-official';
export type UserApiModelType = 'llm' | 'image' | 'video' | 'audio' | 'lipsync';

export interface UserApiProvider {
  id: string;
  name: string;
  baseUrl?: string;
  apiKey?: string;
  hasApiKey?: boolean;
  apiMode?: UserApiMode;
}

export interface UserApiCustomPricing {
  llm?: {
    inputPerMillion?: number;
    outputPerMillion?: number;
  };
  image?: {
    basePrice?: number;
    optionPrices?: Record<string, Record<string, number>>;
  };
  video?: {
    basePrice?: number;
    optionPrices?: Record<string, Record<string, number>>;
  };
}

export interface UserApiModel {
  modelId: string;
  modelKey: string;
  name: string;
  type: UserApiModelType;
  provider: string;
  enabled: boolean;
  price: number;
  priceMin?: number;
  priceMax?: number;
  priceLabel?: string;
  priceInput?: number;
  priceOutput?: number;
  capabilities?: ModelCapabilities;
  customPricing?: UserApiCustomPricing;
}

export interface UserApiDefaultModels {
  analysisModel?: string;
  characterModel?: string;
  locationModel?: string;
  storyboardModel?: string;
  editModel?: string;
  videoModel?: string;
  lipSyncModel?: string;
}

export interface UserApiConfigPayload {
  models: UserApiModel[];
  providers: UserApiProvider[];
  defaultModels: UserApiDefaultModels;
  capabilityDefaults: CapabilitySelections;
  pricingDisplay?: Record<string, unknown>;
}

export interface UpdateUserApiConfigPayload {
  defaultModels?: UserApiDefaultModels;
  capabilityDefaults?: CapabilitySelections;
}

export function getUserApiConfig() {
  return apiRequest<UserApiConfigPayload>('/api/user/api-config');
}

export function updateUserApiConfig(payload: UpdateUserApiConfigPayload) {
  return apiRequest<UserApiConfigPayload>('/api/user/api-config', {
    method: 'PUT',
    body: JSON.stringify(payload),
  });
}

export interface TestUserProviderConnectionPayload {
  provider?: string;
  apiKey?: string;
  baseUrl?: string;
  model?: string;
}

export interface TestUserProviderConnectionResult {
  provider?: string;
  message?: string;
}

export function testUserProviderConnection(payload: TestUserProviderConnectionPayload) {
  return apiRequest<TestUserProviderConnectionResult>('/api/user/api-config/test-connection', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}
