import { useSyncExternalStore } from 'react';

import { hasAuthToken, subscribeAuthToken } from '../api/client';

export function useHasAuthToken(): boolean {
  return useSyncExternalStore(subscribeAuthToken, hasAuthToken, () => false);
}
