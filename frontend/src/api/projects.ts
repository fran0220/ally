import { apiRequest } from './client';

export interface ProjectSummary {
  id: string;
  name: string;
  description: string | null;
  mode: string;
  userId: string;
  createdAt: string;
  updatedAt: string;
  lastAccessedAt: string | null;
}

export interface Pagination {
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
}

export interface ProjectListResponse {
  projects: ProjectSummary[];
  pagination: Pagination;
}

export interface ProjectDataResponse {
  project: {
    id: string;
    name: string;
    description: string | null;
    mode: string;
    userId: string;
    createdAt: string;
    updatedAt: string;
    lastAccessedAt: string | null;
    novelPromotionData: {
      id: string;
      projectId: string;
      analysisModel: string | null;
      imageModel: string | null;
      videoModel: string | null;
      videoRatio: string;
      artStyle: string;
      ttsRate: string;
      episodes: Array<{ id: string; episodeNumber: number; name: string }>;
    } | null;
  };
}

export interface ProjectAssetsResponse {
  characters: Array<{ id: string; name: string }>;
  locations: Array<{ id: string; name: string }>;
}

export interface ProjectMutationInput {
  name: string;
  description?: string;
}

export interface UpdateProjectInput {
  name?: string;
  description?: string;
}

export function listProjects(page: number, pageSize: number, search: string) {
  const params = new URLSearchParams({
    page: String(page),
    pageSize: String(pageSize),
  });
  if (search.trim()) {
    params.set('search', search.trim());
  }
  return apiRequest<ProjectListResponse>(`/api/projects?${params.toString()}`);
}

export function createProject(input: ProjectMutationInput) {
  return apiRequest<{ project: ProjectSummary }>('/api/projects', {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function updateProject(projectId: string, input: UpdateProjectInput) {
  return apiRequest<{ project: ProjectSummary }>(`/api/projects/${projectId}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });
}

export function deleteProject(projectId: string) {
  return apiRequest<{ success: boolean; cosFilesDeleted: number; cosFilesFailed: number }>(
    `/api/projects/${projectId}`,
    {
      method: 'DELETE',
    },
  );
}

export function getProjectData(projectId: string) {
  return apiRequest<ProjectDataResponse>(`/api/projects/${projectId}/data`);
}

export function getProjectAssets(projectId: string) {
  return apiRequest<ProjectAssetsResponse>(`/api/projects/${projectId}/assets`);
}
