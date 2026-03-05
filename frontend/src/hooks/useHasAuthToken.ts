import { useAuthSession } from './useAuthSession';

export function useHasAuthToken(): boolean {
  const { isAuthenticated } = useAuthSession();
  return isAuthenticated;
}
