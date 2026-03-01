import { apiRequest } from './client';

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

export interface NovelCharacter {
  id: string;
  name: string;
  appearances: Array<{ id: string; appearanceIndex: number; imageUrl: string | null }>;
}

export interface NovelLocation {
  id: string;
  name: string;
  images: Array<{ id: string; imageIndex: number; imageUrl: string | null; isSelected: boolean }>;
}

export interface NovelProjectRootResponse {
  project: {
    id: string;
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
    imageModel: string;
    videoModel: string;
    videoRatio: string;
    ttsRate: string;
    globalAssetText: string;
    artStyle: string;
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
    videos: Array<{ panelId: string; storyboardId: string; panelIndex: number; videoUrl: string }>;
  }>(`/api/novel-promotion/${projectId}/video-urls`, {
    method: 'POST',
    body: JSON.stringify({ episodeId }),
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
    }>;
  }>(`/api/novel-promotion/${projectId}/voice-lines?${params.toString()}`);
}
