import type { ParsedModelKey } from './types';

const PROVIDER_INSTANCE_SEPARATOR = ':';
const MODEL_KEY_SEPARATOR = '::';

export function providerBaseKey(providerId: string): string {
  const normalized = providerId.trim();
  if (!normalized) {
    return '';
  }

  const separatorIndex = normalized.indexOf(PROVIDER_INSTANCE_SEPARATOR);
  if (separatorIndex === -1) {
    return normalized;
  }
  return normalized.slice(0, separatorIndex);
}

export function getProviderKey(providerId?: string | null): string {
  if (!providerId) {
    return '';
  }
  return providerBaseKey(providerId);
}

export function modelKey(providerId: string, modelId: string): string {
  return `${providerId.trim()}${MODEL_KEY_SEPARATOR}${modelId.trim()}`;
}

export const encodeModelKey = modelKey;

export function parseModelKeyStrict(key: string | undefined | null): ParsedModelKey | null {
  if (!key) {
    return null;
  }

  const normalized = key.trim();
  const markerIndex = normalized.indexOf(MODEL_KEY_SEPARATOR);
  if (markerIndex <= 0) {
    return null;
  }

  const provider = normalized.slice(0, markerIndex).trim();
  const modelId = normalized.slice(markerIndex + MODEL_KEY_SEPARATOR.length).trim();

  if (!provider || !modelId) {
    return null;
  }

  return { provider, modelId };
}

export function parseModelKey(key: string | undefined | null): ParsedModelKey | null {
  return parseModelKeyStrict(key);
}

export function matchesModelKey(key: string | undefined | null, provider: string, modelId: string): boolean {
  const parsed = parseModelKeyStrict(key);
  if (!parsed) {
    return false;
  }
  return parsed.provider === provider && parsed.modelId === modelId;
}
