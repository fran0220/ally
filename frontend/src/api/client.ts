export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const AUTH_TOKEN_STORAGE_KEY = 'waoowaoo.auth_token';
const AUTH_TOKEN_CHANGE_EVENT = 'waoowaoo:auth-token-changed';

let authTokenCache: string | null = null;
let authTokenLoaded = false;

function normalizeToken(token: string | null | undefined): string | null {
  if (!token) {
    return null;
  }
  const trimmed = token.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function canUseStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readAuthToken(): string | null {
  if (authTokenLoaded) {
    return authTokenCache;
  }

  authTokenLoaded = true;
  if (!canUseStorage()) {
    return authTokenCache;
  }

  try {
    authTokenCache = normalizeToken(window.localStorage.getItem(AUTH_TOKEN_STORAGE_KEY));
  } catch {
    authTokenCache = null;
  }

  return authTokenCache;
}

function emitAuthTokenChanged(): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(AUTH_TOKEN_CHANGE_EVENT));
  }
}

export function hasAuthToken(): boolean {
  return readAuthToken() !== null;
}

export function getAuthToken(): string | null {
  return readAuthToken();
}

export function subscribeAuthToken(listener: () => void): () => void {
  if (typeof window === 'undefined') {
    return () => {
      // no-op in non-browser runtimes
    };
  }

  const onTokenChanged = () => {
    authTokenLoaded = false;
    listener();
  };
  const onStorage = (event: StorageEvent) => {
    if (event.key === AUTH_TOKEN_STORAGE_KEY) {
      onTokenChanged();
    }
  };

  window.addEventListener(AUTH_TOKEN_CHANGE_EVENT, onTokenChanged);
  window.addEventListener('storage', onStorage);

  return () => {
    window.removeEventListener(AUTH_TOKEN_CHANGE_EVENT, onTokenChanged);
    window.removeEventListener('storage', onStorage);
  };
}

export function setAuthToken(token: string | null): void {
  authTokenCache = normalizeToken(token);
  authTokenLoaded = true;

  if (canUseStorage()) {
    try {
      if (authTokenCache) {
        window.localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, authTokenCache);
      } else {
        window.localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
      }
    } catch {
      // Storage can fail in privacy modes; keep in-memory token as the fallback.
    }
  }

  emitAuthTokenChanged();
}

export class ApiClientError extends Error {
  public readonly status: number;
  public readonly payload: unknown;

  constructor(message: string, status: number, payload: unknown) {
    super(message);
    this.status = status;
    this.payload = payload;
  }
}

function parseResponsePayload(response: Response, rawText: string): unknown {
  if (!rawText) {
    return null;
  }

  const contentType = response.headers.get('content-type') ?? '';
  if (contentType.includes('application/json')) {
    try {
      return JSON.parse(rawText) as unknown;
    } catch {
      return { message: 'Invalid JSON response', raw: rawText };
    }
  }

  return rawText;
}

function resolveErrorMessage(status: number, payload: unknown): string {
  if (typeof payload === 'object' && payload !== null) {
    const record = payload as Record<string, unknown>;
    if (typeof record.message === 'string' && record.message.trim()) {
      return record.message;
    }
    if (typeof record.error === 'string' && record.error.trim()) {
      return record.error;
    }
  }
  return `API request failed: ${status}`;
}

function hasScheme(value: string): boolean {
  return /^[a-zA-Z][a-zA-Z\d+.-]*:/.test(value);
}

function shouldSetJsonContentType(init: RequestInit): boolean {
  return Boolean(init.body) && !(init.body instanceof FormData);
}

export function resolveApiUrl(path: string): string {
  return hasScheme(path) ? path : `${API_BASE_URL}${path}`;
}

export function buildAuthHeaders(init: RequestInit = {}): Headers {
  const headers = new Headers(init.headers ?? undefined);

  if (!headers.has('Content-Type') && shouldSetJsonContentType(init)) {
    headers.set('Content-Type', 'application/json');
  }

  if (!headers.has('Authorization')) {
    const authToken = readAuthToken();
    if (authToken) {
      headers.set('Authorization', `Bearer ${authToken}`);
    }
  }

  return headers;
}

export async function fetchWithAuth(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const headers = buildAuthHeaders(init);
  const resolvedInput = typeof input === 'string' && !hasScheme(input) ? resolveApiUrl(input) : input;

  return fetch(resolvedInput, {
    ...init,
    credentials: init.credentials ?? 'include',
    headers,
  });
}

export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetchWithAuth(path, init);

  const rawText = await response.text();
  const payload = parseResponsePayload(response, rawText);

  if (!response.ok) {
    if (response.status === 401) {
      setAuthToken(null);
    }
    throw new ApiClientError(resolveErrorMessage(response.status, payload), response.status, payload);
  }

  return payload as T;
}
