import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';

import { type TaskLifecycleEvent, subscribeTaskEvents } from '../api/sse';
import { queryKeys } from '../lib/query-keys';

interface UseTaskSseOptions {
  projectId: string | null;
  episodeId?: string | null;
  enabled?: boolean;
}

interface TaskSseState {
  connected: boolean;
  events: TaskLifecycleEvent[];
}

function isTerminalEvent(eventType: string): boolean {
  return eventType === 'task.completed' || eventType === 'task.failed' || eventType === 'task.dismissed';
}

export function useTaskSse({ projectId, episodeId, enabled = true }: UseTaskSseOptions): TaskSseState {
  const queryClient = useQueryClient();
  const [connected, setConnected] = useState(false);
  const [events, setEvents] = useState<TaskLifecycleEvent[]>([]);

  useEffect(() => {
    if (!enabled || !projectId) {
      setConnected(false);
      return;
    }

    const unsubscribe = subscribeTaskEvents({
      projectId,
      episodeId,
      onOpen: () => setConnected(true),
      onError: () => setConnected(false),
      onLifecycle: (event) => {
        setEvents((previous) => [event, ...previous].slice(0, 120));

        queryClient.invalidateQueries({ queryKey: queryKeys.tasks.list(projectId, episodeId ?? null) });

        if (isTerminalEvent(event.eventType)) {
          queryClient.invalidateQueries({ queryKey: queryKeys.novel.root(projectId) });
          if (episodeId) {
            queryClient.invalidateQueries({ queryKey: queryKeys.novel.episode(projectId, episodeId) });
            queryClient.invalidateQueries({ queryKey: queryKeys.novel.storyboards(projectId, episodeId) });
          }
          if (projectId === 'global-asset-hub') {
            queryClient.invalidateQueries({ queryKey: queryKeys.assetHub.folders() });
            queryClient.invalidateQueries({ queryKey: queryKeys.assetHub.characters(null) });
            queryClient.invalidateQueries({ queryKey: queryKeys.assetHub.locations(null) });
            queryClient.invalidateQueries({ queryKey: queryKeys.assetHub.voices(null) });
          }
        }
      },
    });

    return () => {
      unsubscribe();
      setConnected(false);
    };
  }, [enabled, episodeId, projectId, queryClient]);

  return useMemo(
    () => ({
      connected,
      events,
    }),
    [connected, events],
  );
}
