import { QueryClient } from '@tanstack/react-query';

import { ApiClientError } from '../api/client';

const MAX_QUERY_RETRIES = 2;

function shouldRetry(failureCount: number, error: unknown): boolean {
  if (failureCount >= MAX_QUERY_RETRIES) {
    return false;
  }

  if (error instanceof TypeError) {
    return true;
  }

  if (error instanceof ApiClientError) {
    if (error.retryable === true) {
      return true;
    }
    if (error.retryable === false) {
      return false;
    }
    return !(error.status >= 400 && error.status < 500);
  }

  return true;
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
