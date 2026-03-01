import { apiRequest } from './client';

export interface TaskRecord {
  id: string;
  userId: string;
  projectId: string;
  episodeId: string | null;
  type: string;
  targetType: string;
  targetId: string;
  status: string;
  progress: number;
  payload: Record<string, unknown> | null;
  result: Record<string, unknown> | null;
  errorCode: string | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TaskListQuery {
  projectId?: string;
  targetType?: string;
  targetId?: string;
  status?: string[];
  type?: string[];
  limit?: number;
}

export function listTasks(query: TaskListQuery) {
  const params = new URLSearchParams();
  if (query.projectId) {
    params.set('projectId', query.projectId);
  }
  if (query.targetType) {
    params.set('targetType', query.targetType);
  }
  if (query.targetId) {
    params.set('targetId', query.targetId);
  }
  if (query.limit !== undefined) {
    params.set('limit', String(query.limit));
  }
  query.status?.forEach((item) => params.append('status', item));
  query.type?.forEach((item) => params.append('type', item));
  const queryString = params.toString();
  const path = queryString ? `/api/tasks?${queryString}` : '/api/tasks';
  return apiRequest<{ tasks: TaskRecord[] }>(path);
}

export function getTask(taskId: string) {
  return apiRequest<{ task: TaskRecord; events: Array<Record<string, unknown>> | null }>(`/api/tasks/${taskId}`);
}
