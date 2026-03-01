export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3001';
const AUTH_TOKEN_STORAGE_KEY = 'waoowaoo.auth_token';

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

export function setAuthToken(token: string | null): void {
  authTokenCache = normalizeToken(token);
  authTokenLoaded = true;

  if (!canUseStorage()) {
    return;
  }

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

export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers ?? undefined);
  if (!headers.has('Content-Type') && init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  if (!headers.has('Authorization')) {
    const authToken = readAuthToken();
    if (authToken) {
      headers.set('Authorization', `Bearer ${authToken}`);
    }
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    credentials: 'include',
    headers,
    ...init,
  });

  const rawText = await response.text();
  const payload = parseResponsePayload(response, rawText);

  if (!response.ok) {
    throw new ApiClientError(resolveErrorMessage(response.status, payload), response.status, payload);
  }

  return payload as T;
}
