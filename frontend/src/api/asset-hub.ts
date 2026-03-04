import { apiRequest } from './client';

export interface AssetFolder {
  id: string;
  user_id?: string;
  userId?: string;
  name: string;
  created_at?: string;
  updated_at?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface AssetCharacterAppearance {
  id: string;
  appearanceIndex: number;
  changeReason: string;
  description: string | null;
  imageUrl: string | null;
  imageUrls: string[];
  selectedIndex: number | null;
}

export interface AssetCharacter {
  id: string;
  folderId: string | null;
  name: string;
  voiceId?: string | null;
  voiceType?: string | null;
  customVoiceUrl: string | null;
  globalVoiceId?: string | null;
  appearances: AssetCharacterAppearance[];
}

export interface AssetLocationImage {
  id: string;
  imageIndex: number;
  imageUrl: string | null;
  description: string | null;
  isSelected: boolean;
}

export interface AssetLocation {
  id: string;
  folderId: string | null;
  name: string;
  summary: string | null;
  images: AssetLocationImage[];
}

export interface AssetVoice {
  id: string;
  folderId: string | null;
  name: string;
  description: string | null;
  voiceType: string;
  customVoiceUrl: string | null;
  language: string;
  gender: string | null;
}

export function listAssetFolders() {
  return apiRequest<{ folders: AssetFolder[] }>('/api/asset-hub/folders');
}

export function createAssetFolder(name: string) {
  return apiRequest<{ success: boolean; folder: AssetFolder }>('/api/asset-hub/folders', {
    method: 'POST',
    body: JSON.stringify({ name }),
  });
}

export function updateAssetFolder(folderId: string, name: string) {
  return apiRequest<{ success: boolean; folder: AssetFolder }>(`/api/asset-hub/folders/${folderId}`, {
    method: 'PATCH',
    body: JSON.stringify({ name }),
  });
}

export function deleteAssetFolder(folderId: string) {
  return apiRequest<{ success: boolean }>(`/api/asset-hub/folders/${folderId}`, {
    method: 'DELETE',
  });
}

export function listAssetCharacters(folderId?: string | null) {
  const params = new URLSearchParams();
  if (folderId) {
    params.set('folderId', folderId);
  }
  const query = params.toString();
  const path = query ? `/api/asset-hub/characters?${query}` : '/api/asset-hub/characters';
  return apiRequest<{ characters: AssetCharacter[] }>(path);
}

export function createAssetCharacter(payload: {
  name: string;
  folderId?: string | null;
  profileData?: Record<string, unknown>;
}) {
  return apiRequest<{ success: boolean; character: AssetCharacter }>('/api/asset-hub/characters', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateAssetCharacter(
  characterId: string,
  payload: Partial<{
    name: string;
    folderId: string | null;
    voiceId: string | null;
    voiceType: string | null;
    customVoiceUrl: string | null;
    globalVoiceId: string | null;
  }>,
) {
  return apiRequest<{ success: boolean; character: AssetCharacter }>(`/api/asset-hub/characters/${characterId}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteAssetCharacter(characterId: string) {
  return apiRequest<{ success: boolean }>(`/api/asset-hub/characters/${characterId}`, {
    method: 'DELETE',
  });
}

export function updateAssetCharacterAppearance(
  characterId: string,
  appearanceIndex: number,
  payload: Partial<{
    description: string;
    selectedIndex: number;
    changeReason: string;
  }>,
) {
  return apiRequest<{ success: boolean }>(
    `/api/asset-hub/characters/${characterId}/appearances/${appearanceIndex}`,
    {
      method: 'PATCH',
      body: JSON.stringify(payload),
    },
  );
}

export function listAssetLocations(folderId?: string | null) {
  const params = new URLSearchParams();
  if (folderId) {
    params.set('folderId', folderId);
  }
  const query = params.toString();
  const path = query ? `/api/asset-hub/locations?${query}` : '/api/asset-hub/locations';
  return apiRequest<{ locations: AssetLocation[] }>(path);
}

export function createAssetLocation(payload: {
  name: string;
  summary?: string;
  folderId?: string | null;
  imageUrl?: string;
  description?: string;
}) {
  return apiRequest<{ success: boolean; location: AssetLocation }>('/api/asset-hub/locations', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateAssetLocation(
  locationId: string,
  payload: Partial<{
    name: string;
    summary: string | null;
    folderId: string | null;
  }>,
) {
  return apiRequest<{ success: boolean; location: AssetLocation }>(`/api/asset-hub/locations/${locationId}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteAssetLocation(locationId: string) {
  return apiRequest<{ success: boolean }>(`/api/asset-hub/locations/${locationId}`, {
    method: 'DELETE',
  });
}

export function listAssetVoices(folderId?: string | null) {
  const params = new URLSearchParams();
  if (folderId) {
    params.set('folderId', folderId);
  }
  const query = params.toString();
  const path = query ? `/api/asset-hub/voices?${query}` : '/api/asset-hub/voices';
  return apiRequest<{ voices: AssetVoice[] }>(path);
}

export function createAssetVoice(payload: {
  name: string;
  description?: string;
  folderId?: string | null;
  voiceType?: string;
  customVoiceUrl?: string;
  language?: string;
}) {
  return apiRequest<{ success: boolean; voice: AssetVoice }>('/api/asset-hub/voices', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateAssetVoice(
  voiceId: string,
  payload: Partial<{
    name: string;
    description: string | null;
    folderId: string | null;
    customVoiceUrl: string | null;
    voiceType: string | null;
  }>,
) {
  return apiRequest<{ success: boolean; voice: AssetVoice }>(`/api/asset-hub/voices/${voiceId}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteAssetVoice(voiceId: string) {
  return apiRequest<{ success: boolean }>(`/api/asset-hub/voices/${voiceId}`, {
    method: 'DELETE',
  });
}

export function bindCharacterVoice(
  characterId: string,
  payload: {
    globalVoiceId?: string | null;
    voiceId?: string | null;
    voiceType?: string | null;
    customVoiceUrl?: string | null;
  },
) {
  return updateAssetCharacter(characterId, {
    globalVoiceId: payload.globalVoiceId ?? null,
    voiceId: payload.voiceId ?? null,
    voiceType: payload.voiceType ?? null,
    customVoiceUrl: payload.customVoiceUrl ?? null,
  });
}
