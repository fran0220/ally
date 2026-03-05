import { buildAuthHeaders, resolveApiUrl } from './client';
import {
  parseHeartbeatPayload,
  parseTaskLifecycleEvent,
  parseTaskStreamEvent,
  type HeartbeatPayload,
  type TaskLifecycleEvent,
  type TaskStreamEvent,
} from './contracts';

export const TASK_LIFECYCLE_EVENT = 'task.lifecycle';
export const TASK_STREAM_EVENT = 'task.stream';
export const HEARTBEAT_EVENT = 'heartbeat';
export type { HeartbeatPayload, TaskLifecycleEvent, TaskStreamEvent } from './contracts';

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

type SseMessageListener = (event: MessageEvent<string>) => void;

interface TaskEventSource {
  onopen: ((event: Event) => void) | null;
  onerror: ((event: Event) => void) | null;
  onmessage: SseMessageListener | null;
  addEventListener(type: string, listener: EventListener): void;
  removeEventListener(type: string, listener: EventListener): void;
  close(): void;
  getLastEventId(): string | null;
}

class FetchTaskEventSource implements TaskEventSource {
  public onopen: ((event: Event) => void) | null = null;
  public onerror: ((event: Event) => void) | null = null;
  public onmessage: SseMessageListener | null = null;

  private readonly abortController = new AbortController();
  private readonly decoder = new TextDecoder();
  private readonly listeners = new Map<string, Set<EventListener>>();
  private readonly url: string;

  private lastEventId: string | null;
  private closed = false;
  private buffer = '';
  private currentEventName = '';
  private currentEventId: string | null = null;
  private currentData: string[] = [];

  constructor(url: string, lastEventId?: string | null) {
    this.url = url;
    this.lastEventId = lastEventId ?? null;
    void this.connect();
  }

  public addEventListener(type: string, listener: EventListener): void {
    const existing = this.listeners.get(type);
    if (existing) {
      existing.add(listener);
      return;
    }

    this.listeners.set(type, new Set([listener]));
  }

  public removeEventListener(type: string, listener: EventListener): void {
    const existing = this.listeners.get(type);
    if (!existing) {
      return;
    }

    existing.delete(listener);
    if (existing.size === 0) {
      this.listeners.delete(type);
    }
  }

  public close(): void {
    if (this.closed) {
      return;
    }

    this.closed = true;
    this.abortController.abort();
  }

  public getLastEventId(): string | null {
    return this.lastEventId;
  }

  private dispatchEvent(type: string, event: Event): void {
    const listeners = this.listeners.get(type);
    if (!listeners) {
      return;
    }

    for (const listener of listeners) {
      listener(event);
    }
  }

  private emitOpen(): void {
    const openEvent = new Event('open');
    this.onopen?.(openEvent);
    this.dispatchEvent('open', openEvent);
  }

  private emitError(): void {
    const errorEvent = new Event('error');
    this.onerror?.(errorEvent);
    this.dispatchEvent('error', errorEvent);
  }

  private dispatchMessageEvent(eventName: string, data: string): void {
    const messageEvent = new MessageEvent<string>(eventName, {
      data,
      lastEventId: this.lastEventId ?? '',
    });

    if (eventName === 'message') {
      this.onmessage?.(messageEvent);
    }
    this.dispatchEvent(eventName, messageEvent);
  }

  private resetCurrentEvent(): void {
    this.currentEventName = '';
    this.currentEventId = null;
    this.currentData = [];
  }

  private flushCurrentEvent(): void {
    if (this.currentEventId !== null) {
      this.lastEventId = this.currentEventId;
    }

    if (this.currentData.length === 0) {
      this.resetCurrentEvent();
      return;
    }

    const eventName = this.currentEventName || 'message';
    const data = this.currentData.join('\n');
    this.dispatchMessageEvent(eventName, data);
    this.resetCurrentEvent();
  }

  private processLine(line: string): void {
    if (line === '') {
      this.flushCurrentEvent();
      return;
    }

    if (line.startsWith(':')) {
      return;
    }

    const colonIndex = line.indexOf(':');
    const field = colonIndex === -1 ? line : line.slice(0, colonIndex);
    let value = colonIndex === -1 ? '' : line.slice(colonIndex + 1);
    if (value.startsWith(' ')) {
      value = value.slice(1);
    }

    switch (field) {
      case 'event':
        this.currentEventName = value;
        break;
      case 'data':
        this.currentData.push(value);
        break;
      case 'id':
        this.currentEventId = value;
        break;
      default:
        break;
    }
  }

  private processChunk(chunk: string): void {
    if (!chunk) {
      return;
    }

    this.buffer += chunk;
    let lineBreakIndex = this.buffer.indexOf('\n');

    while (lineBreakIndex >= 0) {
      let line = this.buffer.slice(0, lineBreakIndex);
      this.buffer = this.buffer.slice(lineBreakIndex + 1);

      if (line.endsWith('\r')) {
        line = line.slice(0, -1);
      }

      this.processLine(line);
      lineBreakIndex = this.buffer.indexOf('\n');
    }
  }

  private buildHeaders(): Headers {
    const headers = buildAuthHeaders();
    headers.set('Accept', 'text/event-stream');
    headers.set('Cache-Control', 'no-cache');

    if (this.lastEventId) {
      headers.set('Last-Event-ID', this.lastEventId);
    }

    return headers;
  }

  private async connect(): Promise<void> {
    try {
      const response = await fetch(this.url, {
        method: 'GET',
        headers: this.buildHeaders(),
        credentials: 'include',
        signal: this.abortController.signal,
        cache: 'no-store',
      });

      if (this.closed) {
        return;
      }

      if (!response.ok || !response.body) {
        this.emitError();
        return;
      }

      this.emitOpen();

      const reader = response.body.getReader();
      try {
        while (!this.closed) {
          const { done, value } = await reader.read();
          if (done) {
            break;
          }

          if (value) {
            this.processChunk(this.decoder.decode(value, { stream: true }));
          }
        }

        const tail = this.decoder.decode();
        if (tail) {
          this.processChunk(tail);
        }

        // Flush pending buffered data if the stream ended without a terminal blank line.
        if (this.buffer.length > 0 || this.currentData.length > 0 || this.currentEventId !== null) {
          this.processChunk('\n\n');
        }
      } finally {
        reader.releaseLock();
      }

      if (!this.closed) {
        this.emitError();
      }
    } catch {
      if (!this.closed && !this.abortController.signal.aborted) {
        this.emitError();
      }
    }
  }
}

function parseJson(raw: string): unknown {
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
}

function parseLifecycleEvent(raw: string): TaskLifecycleEvent | null {
  const parsed = parseJson(raw);
  if (parsed === null) {
    return null;
  }

  try {
    return parseTaskLifecycleEvent(parsed);
  } catch {
    return null;
  }
}

function parseStreamEvent(raw: string): TaskStreamEvent | null {
  const parsed = parseJson(raw);
  if (parsed === null) {
    return null;
  }

  try {
    return parseTaskStreamEvent(parsed);
  } catch {
    return null;
  }
}

function parseHeartbeat(raw: string): HeartbeatPayload | null {
  const parsed = parseJson(raw);
  if (parsed === null) {
    return null;
  }

  try {
    return parseHeartbeatPayload(parsed);
  } catch {
    return null;
  }
}

function buildSsePath(projectId: string, episodeId?: string | null): string {
  const params = new URLSearchParams({ projectId });
  if (episodeId) {
    params.set('episodeId', episodeId);
  }

  const query = params.toString();
  return query ? `/api/sse?${query}` : '/api/sse';
}

function buildSseUrl(projectId: string, episodeId?: string | null): string {
  const path = buildSsePath(projectId, episodeId);
  return resolveApiUrl(path);
}

function createTaskEventSourceWithCursor(
  projectId: string,
  episodeId?: string | null,
  lastEventId?: string | null,
): TaskEventSource | null {
  const url = buildSseUrl(projectId, episodeId);
  try {
    return new FetchTaskEventSource(url, lastEventId);
  } catch {
    return null;
  }
}

export function createTaskEventSource(projectId: string, episodeId?: string | null): EventSource | null {
  return createTaskEventSourceWithCursor(projectId, episodeId) as EventSource | null;
}

export function subscribeTaskEvents(options: TaskEventSubscriptionOptions): () => void {
  let source: TaskEventSource | null = null;
  let reconnectTimer: number | null = null;
  let reconnectDelay = 1_000;
  let lastEventId: string | null = null;
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
      lastEventId = source.getLastEventId();
      source.close();
      source = null;
    }
  };

  const scheduleReconnect = () => {
    if (!autoReconnect || disposed) {
      return;
    }

    clearReconnectTimer();
    reconnectTimer = window.setTimeout(() => {
      reconnectDelay = Math.min(reconnectDelay * 2, maxReconnectDelayMs);
      connect();
    }, reconnectDelay);
  };

  const connect = () => {
    if (disposed) {
      return;
    }

    clearReconnectTimer();
    closeCurrentSource();

    source = createTaskEventSourceWithCursor(options.projectId, options.episodeId, lastEventId);
    if (!source) {
      options.onError?.(new Event('error'));
      scheduleReconnect();
      return;
    }

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

      closeCurrentSource();
      scheduleReconnect();
    };
  };

  connect();

  return () => {
    disposed = true;
    clearReconnectTimer();
    closeCurrentSource();
  };
}
