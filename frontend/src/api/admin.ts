import { apiRequest } from './client';

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
  customPricing?: Record<string, unknown>;
}

export interface AdminAiConfig {
  providers: AdminProvider[];
  models: AdminModel[];
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
