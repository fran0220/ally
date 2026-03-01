import { apiRequest } from './client';

export function listRuns() {
  return apiRequest<{ runs: Array<Record<string, unknown>> }>('/api/runs');
}
