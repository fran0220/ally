import { useMemo } from 'react';

import { useAuthSession } from './useAuthSession';

export interface CurrentUser {
  username: string | null;
  role: string | null;
  userId: string | null;
  isAdmin: boolean;
}

const EMPTY_USER: CurrentUser = {
  username: null,
  role: null,
  userId: null,
  isAdmin: false,
};

export function useCurrentUser(): CurrentUser {
  const { user } = useAuthSession();

  return useMemo<CurrentUser>(() => {
    if (!user) {
      return EMPTY_USER;
    }

    return {
      username: user.name,
      role: user.role,
      userId: user.id,
      isAdmin: user.role === 'admin',
    };
  }, [user]);
}
