import { useEffect, useState } from 'react';
import { QueryClientProvider } from '@tanstack/react-query';

import { subscribeAuthToken } from '../../api/client';
import { createQueryClient } from '../../lib/query-client';
import { queryKeys } from '../../lib/query-keys';

export function QueryProvider({ children }: { children: React.ReactNode }) {
  const [client] = useState(createQueryClient);

  useEffect(() => {
    return subscribeAuthToken(() => {
      void client.invalidateQueries({ queryKey: queryKeys.auth.session() });
    });
  }, [client]);

  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
