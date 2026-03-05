import { apiRequest, apiRequestBlob } from './client';

export interface NovelEpisode {
  id: string;
  novelPromotionProjectId: string;
  episodeNumber: number;
  name: string;
  description: string | null;
  novelText: string | null;
  audioUrl: string | null;
  audioMediaId: string | null;
  srtContent: string | null;
  speakerVoices: Record<string, unknown> | null;
  createdAt: string;
  updatedAt: string;
}

export interface NovelCharacterAppearance {
  id: string;
  characterId?: string;
  appearanceIndex: number;
  changeReason?: string;
  description: string | null;
  descriptions?: unknown;
  imageUrl: string | null;
  imageUrls: string[];
  selectedIndex: number | null;
}

export interface NovelCharacter {
  id: string;
  novelPromotionProjectId?: string;
  name: string;
  aliases?: unknown;
  profileData?: unknown;
  profileConfirmed?: boolean;
  customVoiceUrl?: string | null;
  customVoiceMediaId?: string | null;
  voiceId?: string | null;
  voiceType?: string | null;
  introduction?: string | null;
  sourceGlobalCharacterId?: string | null;
  appearances: NovelCharacterAppearance[];
  createdAt?: string;
  updatedAt?: string;
}

export interface NovelLocationImage {
  id: string;
  locationId?: string;
  imageIndex: number;
  description: string | null;
  imageUrl: string | null;
  imageMediaId?: string | null;
  isSelected: boolean;
}

export interface NovelLocation {
  id: string;
  novelPromotionProjectId?: string;
  name: string;
  summary: string | null;
  sourceGlobalLocationId?: string | null;
  selectedImageId?: string | null;
  images: NovelLocationImage[];
  createdAt?: string;
  updatedAt?: string;
}

export interface NovelTaskSubmitResponse {
  success?: boolean;
  async?: boolean;
  taskId?: string;
  taskIds?: string[];
  total?: number;
  status?: string;
  deduped?: boolean;
}

export interface NovelProjectRootResponse {
  project: {
    id: string;
    name: string | null;
    novelPromotionData: {
      id: string;
      projectId: string;
      analysisModel: string | null;
      imageModel: string | null;
      videoModel: string | null;
      videoRatio: string;
      ttsRate: string;
      globalAssetText: string | null;
      artStyle: string;
      workflowMode: string;
      imageResolution: string;
      videoResolution: string;
      importStatus: string | null;
      episodes: NovelEpisode[];
      characters: NovelCharacter[];
      locations: NovelLocation[];
    };
  };
  capabilityOverrides: Record<string, unknown>;
}

export interface NovelEditorResponse {
  projectData: Record<string, unknown> | null;
  id: string | null;
  episodeId: string;
  renderStatus: string | null;
  outputUrl: string | null;
  updatedAt: string | null;
}

export function getNovelProject(projectId: string) {
  return apiRequest<NovelProjectRootResponse>(`/api/novel-promotion/${projectId}`);
}

export function updateNovelProject(
  projectId: string,
  payload: Partial<{
    analysisModel: string;
    characterModel: string;
    locationModel: string;
    storyboardModel: string;
    editModel: string;
    imageModel: string;
    videoModel: string;
    videoRatio: string;
    ttsRate: string;
    globalAssetText: string;
    artStyle: string;
    workflowMode: string;
    imageResolution: string;
    videoResolution: string;
  }>,
) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function listEpisodes(projectId: string) {
  return apiRequest<{ episodes: NovelEpisode[] }>(`/api/novel-promotion/${projectId}/episodes`);
}

export function createEpisode(projectId: string, payload: Partial<NovelEpisode> & { name: string }) {
  return apiRequest<{ episode: NovelEpisode }>(`/api/novel-promotion/${projectId}/episodes`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function getEpisode(projectId: string, episodeId: string) {
  return apiRequest<{ episode: NovelEpisode }>(`/api/novel-promotion/${projectId}/episodes/${episodeId}`);
}

export function updateEpisode(projectId: string, episodeId: string, payload: Partial<NovelEpisode>) {
  return apiRequest<{ episode: NovelEpisode }>(`/api/novel-promotion/${projectId}/episodes/${episodeId}`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteEpisode(projectId: string, episodeId: string) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/episodes/${episodeId}`, {
    method: 'DELETE',
  });
}

export function listStoryboards(projectId: string, episodeId: string) {
  const params = new URLSearchParams({ episodeId });
  return apiRequest<{ storyboards: Array<Record<string, unknown>> }>(
    `/api/novel-promotion/${projectId}/storyboards?${params.toString()}`,
  );
}

export function getEditorProject(projectId: string, episodeId: string) {
  const params = new URLSearchParams({ episodeId });
  return apiRequest<NovelEditorResponse>(`/api/novel-promotion/${projectId}/editor?${params.toString()}`);
}

export function saveEditorProject(
  projectId: string,
  episodeId: string,
  projectData: Record<string, unknown>,
  renderStatus?: string,
  outputUrl?: string,
) {
  return apiRequest<{ success: boolean; id: string; updatedAt: string }>(`/api/novel-promotion/${projectId}/editor`, {
    method: 'PUT',
    body: JSON.stringify({ episodeId, projectData, renderStatus, outputUrl }),
  });
}

export function listVideoUrls(projectId: string, episodeId: string) {
  return apiRequest<{
    videos: Array<{
      index: number;
      fileName: string;
      panelId: string;
      storyboardId: string;
      panelIndex: number;
      isLipSync?: boolean;
      sourceVideoUrl?: string;
      videoUrl: string;
    }>;
  }>(`/api/novel-promotion/${projectId}/video-urls`, {
    method: 'POST',
    body: JSON.stringify({ episodeId }),
  });
}

export function splitEpisodesByMarkers(projectId: string, content: string) {
  return apiRequest<{
    success: boolean;
    episodes: NovelEpisode[];
    markerType: string;
    method: string;
  }>(`/api/novel-promotion/${projectId}/episodes/split-by-markers`, {
    method: 'POST',
    body: JSON.stringify({ content }),
  });
}

export function listVoiceLines(projectId: string, episodeId: string) {
  const params = new URLSearchParams({ episodeId });
  return apiRequest<{
    voiceLines: Array<{
      id: string;
      lineIndex: number;
      speaker: string;
      content: string;
      audioUrl: string | null;
      matchedPanelId?: string | null;
      matchedStoryboardId?: string | null;
      matchedPanelIndex?: number | null;
      emotionPrompt?: string | null;
      emotionStrength?: number | null;
    }>;
  }>(`/api/novel-promotion/${projectId}/voice-lines?${params.toString()}`);
}

export function triggerStoryToScript(
  projectId: string,
  payload: {
    episodeId: string;
    content: string;
    model?: string;
    temperature?: number;
    reasoning?: boolean;
    reasoningEffort?: 'minimal' | 'low' | 'medium' | 'high';
    async?: boolean;
    displayMode?: 'detail' | 'summary';
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/story-to-script-stream`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerScriptToStoryboard(
  projectId: string,
  payload: {
    episodeId: string;
    model?: string;
    temperature?: number;
    reasoning?: boolean;
    reasoningEffort?: 'minimal' | 'low' | 'medium' | 'high';
    async?: boolean;
    displayMode?: 'detail' | 'summary';
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/script-to-storyboard-stream`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerAnalyzeGlobalAssets(projectId: string, payload: { async?: boolean } = {}) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/analyze-global`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerVoiceAnalyze(
  projectId: string,
  payload: {
    episodeId: string;
    async?: boolean;
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/voice-analyze`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerVoiceGenerate(
  projectId: string,
  payload: {
    episodeId: string;
    lineId?: string;
    all?: boolean;
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/voice-generate`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerVideoGenerate(
  projectId: string,
  payload: {
    episodeId: string;
    videoModel: string;
    all?: boolean;
    storyboardId?: string;
    panelIndex?: number;
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/generate-video`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function triggerProjectAssetImageGenerate(
  projectId: string,
  payload: {
    type: 'character' | 'location';
    id: string;
    appearanceId?: string;
    imageIndex?: number;
    async?: boolean;
    meta?: {
      locale?: string;
    };
  },
) {
  return apiRequest<NovelTaskSubmitResponse>(`/api/novel-promotion/${projectId}/generate-image`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function createProjectCharacter(
  projectId: string,
  payload: {
    name: string;
    aliases?: unknown;
    profileData?: unknown;
    introduction?: string;
  },
) {
  return apiRequest<{ success: boolean; character: { id: string } }>(`/api/novel-promotion/${projectId}/character`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateProjectCharacter(
  projectId: string,
  payload: {
    characterId: string;
    name?: string;
    introduction?: string;
    aliases?: unknown;
    profileData?: unknown;
    voiceId?: string | null;
    voiceType?: string | null;
    customVoiceUrl?: string | null;
    customVoiceMediaId?: string | null;
  },
) {
  return apiRequest<{ success: boolean; character: { id: string } }>(`/api/novel-promotion/${projectId}/character`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteProjectCharacter(projectId: string, characterId: string) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/character`, {
    method: 'DELETE',
    body: JSON.stringify({ characterId }),
  });
}

export function createProjectLocation(
  projectId: string,
  payload: {
    name: string;
    summary?: string;
    description?: string;
    imageUrl?: string;
  },
) {
  return apiRequest<{ success: boolean; location: { id: string } }>(`/api/novel-promotion/${projectId}/location`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function updateProjectLocation(
  projectId: string,
  payload: {
    locationId: string;
    name?: string;
    summary?: string;
    selectedImageId?: string;
  },
) {
  return apiRequest<{ success: boolean; location: { id: string } }>(`/api/novel-promotion/${projectId}/location`, {
    method: 'PATCH',
    body: JSON.stringify(payload),
  });
}

export function deleteProjectLocation(projectId: string, locationId: string) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/location`, {
    method: 'DELETE',
    body: JSON.stringify({ locationId }),
  });
}

export function copyProjectAssetFromGlobal(
  projectId: string,
  payload: {
    type: 'character' | 'location' | 'voice';
    targetId: string;
    globalAssetId: string;
  },
) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/copy-from-global`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function selectProjectCharacterImage(
  projectId: string,
  payload: {
    appearanceId: string;
    selectedIndex: number;
  },
) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/select-character-image`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

export function selectProjectLocationImage(
  projectId: string,
  payload: {
    locationId: string;
    selectedIndex: number;
  },
) {
  return apiRequest<{ success: boolean }>(`/api/novel-promotion/${projectId}/select-location-image`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
}

async function downloadNovelZip(path: string, init?: RequestInit): Promise<Blob> {
  return apiRequestBlob(path, init);
}

export async function downloadProjectImagesZip(projectId: string, episodeId?: string) {
  const params = new URLSearchParams();
  if (episodeId) {
    params.set('episodeId', episodeId);
  }
  const suffix = params.toString();
  const path = suffix
    ? `/api/novel-promotion/${projectId}/download-images?${suffix}`
    : `/api/novel-promotion/${projectId}/download-images`;
  return downloadNovelZip(path);
}

export async function downloadProjectVideosZip(
  projectId: string,
  payload: {
    episodeId?: string;
    panelPreferences?: Record<string, boolean>;
  },
) {
  return downloadNovelZip(`/api/novel-promotion/${projectId}/download-videos`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
}

export async function downloadProjectVoicesZip(projectId: string, episodeId?: string) {
  const params = new URLSearchParams();
  if (episodeId) {
    params.set('episodeId', episodeId);
  }
  const suffix = params.toString();
  const path = suffix
    ? `/api/novel-promotion/${projectId}/download-voices?${suffix}`
    : `/api/novel-promotion/${projectId}/download-voices`;
  return downloadNovelZip(path);
}
