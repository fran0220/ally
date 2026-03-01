import { apiRequest, setAuthToken } from './client';

export interface AuthUser {
  id: string;
  name: string;
  role: 'admin' | 'user';
}

export interface AuthPayload {
  token: string;
  user: AuthUser;
}

export interface RegisterPayload {
  message: string;
  token: string;
  user: AuthUser;
}

export async function login(username: string, password: string): Promise<AuthPayload> {
  const payload = await apiRequest<AuthPayload>('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });
  setAuthToken(payload.token);
  return payload;
}

export async function register(name: string, password: string): Promise<RegisterPayload> {
  const payload = await apiRequest<RegisterPayload>('/api/auth/register', {
    method: 'POST',
    body: JSON.stringify({ name, password }),
  });
  setAuthToken(payload.token);
  return payload;
}

export async function refresh(): Promise<AuthPayload> {
  const payload = await apiRequest<AuthPayload>('/api/auth/refresh', { method: 'POST' });
  setAuthToken(payload.token);
  return payload;
}

export function clearSessionToken(): void {
  setAuthToken(null);
}
