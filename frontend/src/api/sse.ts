import { API_BASE_URL } from './client';

export const TASK_LIFECYCLE_EVENT = 'task.lifecycle';
export const TASK_STREAM_EVENT = 'task.stream';
export const HEARTBEAT_EVENT = 'heartbeat';

export interface TaskLifecycleEvent {
  id: string;
  type: 'task.lifecycle';
  taskId: string;
  projectId: string;
  userId: string;
  eventType: string;
  taskType?: string;
  targetType?: string;
  targetId?: string;
  episodeId?: string;
  payload: Record<string, unknown>;
  ts: string;
}

export interface TaskStreamEvent {
  id: string;
  type: 'task.stream';
  taskId: string;
  projectId: string;
  userId: string;
  eventType: string;
  taskType?: string;
  targetType?: string;
  targetId?: string;
  episodeId?: string;
  payload: Record<string, unknown>;
  ts: string;
}

export interface HeartbeatPayload {
  ts: string;
}

export interface TaskEventSubscriptionOptions {
  projectId: string;
  episodeId?: string | null;
  autoReconnect?: boolean;
  maxReconnectDelayMs?: number;
  onLifecycle: (event: TaskLifecycleEvent) => void;
  onStream?: (event: TaskStreamEvent) => void;
  onHeartbeat?: (payload: HeartbeatPayload) => void;
  onOpen?: () => void;
  onError?: (error: Event) => void;
}

function parseJson(raw: string): unknown {
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseTaskEvent(
  raw: string,
  expectedType: typeof TASK_LIFECYCLE_EVENT | typeof TASK_STREAM_EVENT,
): TaskLifecycleEvent | TaskStreamEvent | null {
  const parsed = parseJson(raw);
  if (!isObject(parsed)) {
    return null;
  }

  if (parsed.type !== expectedType || typeof parsed.taskId !== 'string') {
    return null;
  }

  const payload = isObject(parsed.payload) ? parsed.payload : {};
  return {
    id: typeof parsed.id === 'string' ? parsed.id : '',
    type: expectedType,
    taskId: parsed.taskId,
    projectId: typeof parsed.projectId === 'string' ? parsed.projectId : '',
    userId: typeof parsed.userId === 'string' ? parsed.userId : '',
    eventType: typeof parsed.eventType === 'string' ? parsed.eventType : '',
    taskType: typeof parsed.taskType === 'string' ? parsed.taskType : undefined,
    targetType: typeof parsed.targetType === 'string' ? parsed.targetType : undefined,
    targetId: typeof parsed.targetId === 'string' ? parsed.targetId : undefined,
    episodeId: typeof parsed.episodeId === 'string' ? parsed.episodeId : undefined,
    payload,
    ts: typeof parsed.ts === 'string' ? parsed.ts : new Date().toISOString(),
  };
}

function parseLifecycleEvent(raw: string): TaskLifecycleEvent | null {
  const parsed = parseTaskEvent(raw, TASK_LIFECYCLE_EVENT);
  return parsed as TaskLifecycleEvent | null;
}

function parseStreamEvent(raw: string): TaskStreamEvent | null {
  const parsed = parseTaskEvent(raw, TASK_STREAM_EVENT);
  return parsed as TaskStreamEvent | null;
}

function parseHeartbeat(raw: string): HeartbeatPayload | null {
  const parsed = parseJson(raw);
  if (!isObject(parsed) || typeof parsed.ts !== 'string') {
    return null;
  }
  return { ts: parsed.ts };
}

function buildSseUrl(projectId: string, episodeId?: string | null): string {
  const url = new URL('/api/sse', API_BASE_URL);
  url.searchParams.set('projectId', projectId);
  if (episodeId) {
    url.searchParams.set('episodeId', episodeId);
  }
  return url.toString();
}

export function createTaskEventSource(projectId: string, episodeId?: string | null): EventSource {
  const url = buildSseUrl(projectId, episodeId);
  return new EventSource(url.toString(), { withCredentials: true });
}

export function subscribeTaskEvents(options: TaskEventSubscriptionOptions): () => void {
  let source: EventSource | null = null;
  let reconnectTimer: number | null = null;
  let reconnectDelay = 1_000;
  let disposed = false;

  const autoReconnect = options.autoReconnect ?? true;
  const maxReconnectDelayMs = options.maxReconnectDelayMs ?? 15_000;

  const clearReconnectTimer = () => {
    if (reconnectTimer !== null) {
      window.clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };

  const closeCurrentSource = () => {
    if (source) {
      source.close();
      source = null;
    }
  };

  const connect = () => {
    if (disposed) {
      return;
    }

    clearReconnectTimer();
    closeCurrentSource();

    source = createTaskEventSource(options.projectId, options.episodeId);

    source.onopen = () => {
      reconnectDelay = 1_000;
      options.onOpen?.();
    };

    const lifecycleHandler = (event: MessageEvent<string>) => {
      const payload = parseLifecycleEvent(event.data);
      if (payload) {
        options.onLifecycle(payload);
      }
    };

    const streamHandler = (event: MessageEvent<string>) => {
      const payload = parseStreamEvent(event.data);
      if (payload) {
        options.onStream?.(payload);
      }
    };

    const heartbeatHandler = (event: MessageEvent<string>) => {
      const payload = parseHeartbeat(event.data);
      if (payload) {
        options.onHeartbeat?.(payload);
      }
    };

    source.onmessage = (event: MessageEvent<string>) => {
      lifecycleHandler(event);
      streamHandler(event);
    };
    source.addEventListener(TASK_LIFECYCLE_EVENT, lifecycleHandler as EventListener);
    source.addEventListener(TASK_STREAM_EVENT, streamHandler as EventListener);
    source.addEventListener(HEARTBEAT_EVENT, heartbeatHandler as EventListener);

    source.onerror = (error) => {
      options.onError?.(error);

      if (!autoReconnect || disposed) {
        return;
      }

      closeCurrentSource();
      clearReconnectTimer();
      reconnectTimer = window.setTimeout(() => {
        reconnectDelay = Math.min(reconnectDelay * 2, maxReconnectDelayMs);
        connect();
      }, reconnectDelay);
    };
  };

  connect();

  return () => {
    disposed = true;
    clearReconnectTimer();
    closeCurrentSource();
  };
}
