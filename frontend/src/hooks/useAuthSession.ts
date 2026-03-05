import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { getSession, type AuthUser } from '../api/auth';
import { isUnauthorizedApiError } from '../api/client';
import { queryKeys } from '../lib/query-keys';

export interface AuthSessionState {
  user: AuthUser | null;
  isAuthenticated: boolean;
  isBootstrapping: boolean;
}

async function fetchAuthSession(): Promise<AuthUser | null> {
  try {
    const payload = await getSession();
    return payload.user;
  } catch (error) {
    if (isUnauthorizedApiError(error)) {
      return null;
    }
    throw error;
  }
}

export function useAuthSession(): AuthSessionState {
  const sessionQuery = useQuery({
    queryKey: queryKeys.auth.session(),
    queryFn: fetchAuthSession,
  });

  return useMemo<AuthSessionState>(() => {
    const user = sessionQuery.data ?? null;

    return {
      user,
      isAuthenticated: user !== null,
      isBootstrapping: sessionQuery.fetchStatus === 'fetching' && user === null,
    };
  }, [sessionQuery.data, sessionQuery.fetchStatus]);
}
