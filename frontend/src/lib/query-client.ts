import { QueryClient } from '@tanstack/react-query';

import { ApiClientError } from '../api/client';

function shouldRetry(failureCount: number, error: unknown): boolean {
  if (error instanceof ApiClientError && error.status >= 400 && error.status < 500) {
    return false;
  }
  return failureCount < 2;
}

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 8_000,
        gcTime: 10 * 60 * 1_000,
        refetchOnWindowFocus: true,
        refetchOnReconnect: true,
        retry: shouldRetry,
        retryDelay: (attempt) => Math.min(1_000 * 2 ** attempt, 6_000),
      },
      mutations: {
        retry: 0,
      },
    },
  });
}
