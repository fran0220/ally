import {
  parseAuthPayload,
  parseRegisterPayload,
  parseSessionPayload,
  parseSuccessResponse,
  type AuthPayload,
  type RegisterPayload,
  type SessionPayload,
} from './contracts';
import { apiRequestWithContract, setAuthToken } from './client';

export type { AuthUser } from './contracts';

export async function login(username: string, password: string): Promise<AuthPayload> {
  const payload = await apiRequestWithContract('/api/auth/login', parseAuthPayload, {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });
  setAuthToken(payload.token);
  return payload;
}

export async function register(name: string, password: string): Promise<RegisterPayload> {
  const payload = await apiRequestWithContract('/api/auth/register', parseRegisterPayload, {
    method: 'POST',
    body: JSON.stringify({ name, password }),
  });
  setAuthToken(payload.token);
  return payload;
}

export async function refresh(): Promise<AuthPayload> {
  const payload = await apiRequestWithContract('/api/auth/refresh', parseAuthPayload, {
    method: 'POST',
  });
  setAuthToken(payload.token);
  return payload;
}

export async function getSession(): Promise<SessionPayload> {
  return apiRequestWithContract('/api/auth/session', parseSessionPayload);
}

export async function logout(): Promise<void> {
  try {
    await apiRequestWithContract('/api/auth/logout', parseSuccessResponse, {
      method: 'POST',
    });
  } finally {
    setAuthToken(null);
  }
}

export function clearSessionToken(): void {
  setAuthToken(null);
}
