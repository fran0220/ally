import { apiRequest } from './client';

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
