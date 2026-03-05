import {
  subscribeTaskEvents,
  type HeartbeatPayload,
  type TaskEventSubscriptionOptions,
  type TaskLifecycleEvent,
  type TaskStreamEvent,
} from '@/api/sse';

type SharedListener = {
  onLifecycle: (event: TaskLifecycleEvent) => void;
  onStream?: (event: TaskStreamEvent) => void;
  onHeartbeat?: (payload: HeartbeatPayload) => void;
  onOpen?: () => void;
  onError?: (error: Event) => void;
};

type SharedChannel = {
  listeners: Set<SharedListener>;
  unsubscribe: () => void;
  isOpen: boolean;
};

const channels = new Map<string, SharedChannel>();

function channelKey(projectId: string, episodeId?: string | null): string {
  return `${projectId}:${episodeId ?? 'all'}`;
}

export function subscribeSharedTaskEvents(options: TaskEventSubscriptionOptions): () => void {
  const key = channelKey(options.projectId, options.episodeId);
  let channel = channels.get(key);

  if (!channel) {
    const listeners = new Set<SharedListener>();
    const nextChannel: SharedChannel = {
      listeners,
      unsubscribe: () => {
        // Assign after subscribeTaskEvents returns.
      },
      isOpen: false,
    };

    const unsubscribe = subscribeTaskEvents({
      projectId: options.projectId,
      episodeId: options.episodeId,
      autoReconnect: options.autoReconnect,
      maxReconnectDelayMs: options.maxReconnectDelayMs,
      onLifecycle: (event) => {
        for (const listener of listeners) {
          listener.onLifecycle(event);
        }
      },
      onStream: (event) => {
        for (const listener of listeners) {
          listener.onStream?.(event);
        }
      },
      onHeartbeat: (payload) => {
        for (const listener of listeners) {
          listener.onHeartbeat?.(payload);
        }
      },
      onOpen: () => {
        nextChannel.isOpen = true;
        for (const listener of listeners) {
          listener.onOpen?.();
        }
      },
      onError: (error) => {
        nextChannel.isOpen = false;
        for (const listener of listeners) {
          listener.onError?.(error);
        }
      },
    });

    nextChannel.unsubscribe = unsubscribe;
    channel = nextChannel;
    channels.set(key, channel);
  }

  const listener: SharedListener = {
    onLifecycle: options.onLifecycle,
    onStream: options.onStream,
    onHeartbeat: options.onHeartbeat,
    onOpen: options.onOpen,
    onError: options.onError,
  };

  channel.listeners.add(listener);
  if (channel.isOpen) {
    listener.onOpen?.();
  }

  return () => {
    const current = channels.get(key);
    if (!current) {
      return;
    }
    current.listeners.delete(listener);
    if (current.listeners.size === 0) {
      current.unsubscribe();
      channels.delete(key);
    }
  };
}
