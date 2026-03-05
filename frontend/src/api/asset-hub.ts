import {
  parseAssetCharacterListResponse,
  parseAssetCharacterMutationResponse,
  parseAssetFolderListResponse,
  parseAssetFolderMutationResponse,
  parseAssetLocationListResponse,
  parseAssetLocationMutationResponse,
  parseAssetVoiceListResponse,
  parseAssetVoiceMutationResponse,
  parseSuccessResponse,
} from './contracts';
import { apiRequestWithContract } from './client';

export type {
  AssetCharacter,
  AssetCharacterAppearance,
  AssetFolder,
  AssetLocation,
  AssetLocationImage,
  AssetVoice,
} from './contracts';

function buildListPath(basePath: string, folderId?: string | null): string {
  const params = new URLSearchParams();
  if (folderId) {
    params.set('folderId', folderId);
  }

  const query = params.toString();
  return query ? `${basePath}?${query}` : basePath;
}

export function listAssetFolders() {
  return apiRequestWithContract(
    '/api/asset-hub/folders',
    parseAssetFolderListResponse,
  );
}

export function createAssetFolder(name: string) {
  return apiRequestWithContract(
    '/api/asset-hub/folders',
    parseAssetFolderMutationResponse,
    {
      method: 'POST',
      body: JSON.stringify({ name }),
    },
  );
}

export function updateAssetFolder(folderId: string, name: string) {
  return apiRequestWithContract(
    `/api/asset-hub/folders/${folderId}`,
    parseAssetFolderMutationResponse,
    {
      method: 'PATCH',
      body: JSON.stringify({ name }),
    },
  );
}

export function deleteAssetFolder(folderId: string) {
  return apiRequestWithContract(
    `/api/asset-hub/folders/${folderId}`,
    parseSuccessResponse,
    {
      method: 'DELETE',
    },
  );
}

export function listAssetCharacters(folderId?: string | null) {
  return apiRequestWithContract(
    buildListPath('/api/asset-hub/characters', folderId),
    parseAssetCharacterListResponse,
  );
}

export function createAssetCharacter(payload: {
  name: string;
  folderId?: string | null;
  profileData?: Record<string, unknown>;
}) {
  return apiRequestWithContract(
    '/api/asset-hub/characters',
    parseAssetCharacterMutationResponse,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
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
  return apiRequestWithContract(
    `/api/asset-hub/characters/${characterId}`,
    parseAssetCharacterMutationResponse,
    {
      method: 'PATCH',
      body: JSON.stringify(payload),
    },
  );
}

export function deleteAssetCharacter(characterId: string) {
  return apiRequestWithContract(
    `/api/asset-hub/characters/${characterId}`,
    parseSuccessResponse,
    {
      method: 'DELETE',
    },
  );
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
  return apiRequestWithContract(
    `/api/asset-hub/characters/${characterId}/appearances/${appearanceIndex}`,
    parseSuccessResponse,
    {
      method: 'PATCH',
      body: JSON.stringify(payload),
    },
  );
}

export function listAssetLocations(folderId?: string | null) {
  return apiRequestWithContract(
    buildListPath('/api/asset-hub/locations', folderId),
    parseAssetLocationListResponse,
  );
}

export function createAssetLocation(payload: {
  name: string;
  summary?: string;
  folderId?: string | null;
  imageUrl?: string;
  description?: string;
}) {
  return apiRequestWithContract(
    '/api/asset-hub/locations',
    parseAssetLocationMutationResponse,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
}

export function updateAssetLocation(
  locationId: string,
  payload: Partial<{
    name: string;
    summary: string | null;
    folderId: string | null;
  }>,
) {
  return apiRequestWithContract(
    `/api/asset-hub/locations/${locationId}`,
    parseAssetLocationMutationResponse,
    {
      method: 'PATCH',
      body: JSON.stringify(payload),
    },
  );
}

export function deleteAssetLocation(locationId: string) {
  return apiRequestWithContract(
    `/api/asset-hub/locations/${locationId}`,
    parseSuccessResponse,
    {
      method: 'DELETE',
    },
  );
}

export function listAssetVoices(folderId?: string | null) {
  return apiRequestWithContract(
    buildListPath('/api/asset-hub/voices', folderId),
    parseAssetVoiceListResponse,
  );
}

export function createAssetVoice(payload: {
  name: string;
  description?: string;
  folderId?: string | null;
  voiceType?: string;
  customVoiceUrl?: string;
  language?: string;
}) {
  return apiRequestWithContract(
    '/api/asset-hub/voices',
    parseAssetVoiceMutationResponse,
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  );
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
  return apiRequestWithContract(
    `/api/asset-hub/voices/${voiceId}`,
    parseAssetVoiceMutationResponse,
    {
      method: 'PATCH',
      body: JSON.stringify(payload),
    },
  );
}

export function deleteAssetVoice(voiceId: string) {
  return apiRequestWithContract(
    `/api/asset-hub/voices/${voiceId}`,
    parseSuccessResponse,
    {
      method: 'DELETE',
    },
  );
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
