export class ApiContractViolationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ApiContractViolationError';
  }
}

function fail(path: string, message: string): never {
  throw new ApiContractViolationError(`${path} ${message}`);
}

function asRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    fail(path, 'must be an object');
  }
  return value as Record<string, unknown>;
}

function readRequiredValue(
  record: Record<string, unknown>,
  key: string,
  path: string,
): unknown {
  if (!(key in record)) {
    fail(`${path}.${key}`, 'is required');
  }
  return record[key];
}

function readString(
  record: Record<string, unknown>,
  key: string,
  path: string,
): string {
  const value = readRequiredValue(record, key, path);
  if (typeof value !== 'string') {
    fail(`${path}.${key}`, 'must be a string');
  }
  return value;
}

function readNonEmptyString(
  record: Record<string, unknown>,
  key: string,
  path: string,
): string {
  const value = readString(record, key, path).trim();
  if (!value) {
    fail(`${path}.${key}`, 'must be a non-empty string');
  }
  return value;
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
  path: string,
): string | undefined {
  const value = record[key];
  if (value === undefined || value === null) {
    return undefined;
  }
  if (typeof value !== 'string') {
    fail(`${path}.${key}`, 'must be a string when present');
  }
  return value;
}

function readNullableString(
  record: Record<string, unknown>,
  key: string,
  path: string,
): string | null {
  const value = readRequiredValue(record, key, path);
  if (value === null) {
    return null;
  }
  if (typeof value !== 'string') {
    fail(`${path}.${key}`, 'must be a string or null');
  }
  return value;
}

function readInteger(
  record: Record<string, unknown>,
  key: string,
  path: string,
): number {
  const value = readRequiredValue(record, key, path);
  if (typeof value !== 'number' || !Number.isInteger(value)) {
    fail(`${path}.${key}`, 'must be an integer');
  }
  return value;
}

function readNullableInteger(
  record: Record<string, unknown>,
  key: string,
  path: string,
): number | null {
  const value = readRequiredValue(record, key, path);
  if (value === null) {
    return null;
  }
  if (typeof value !== 'number' || !Number.isInteger(value)) {
    fail(`${path}.${key}`, 'must be an integer or null');
  }
  return value;
}

function readBoolean(
  record: Record<string, unknown>,
  key: string,
  path: string,
): boolean {
  const value = readRequiredValue(record, key, path);
  if (typeof value !== 'boolean') {
    fail(`${path}.${key}`, 'must be a boolean');
  }
  return value;
}

function readArray(
  record: Record<string, unknown>,
  key: string,
  path: string,
): unknown[] {
  const value = readRequiredValue(record, key, path);
  if (!Array.isArray(value)) {
    fail(`${path}.${key}`, 'must be an array');
  }
  return value;
}

function readObject(
  record: Record<string, unknown>,
  key: string,
  path: string,
): Record<string, unknown> {
  const value = readRequiredValue(record, key, path);
  return asRecord(value, `${path}.${key}`);
}

function readStringArray(
  record: Record<string, unknown>,
  key: string,
  path: string,
): string[] {
  const array = readArray(record, key, path);
  return array.map((item, index) => {
    if (typeof item !== 'string') {
      fail(`${path}.${key}[${index}]`, 'must be a string');
    }
    return item;
  });
}

export type AuthRole = 'admin' | 'user';

export interface AuthUser {
  id: string;
  name: string;
  role: AuthRole;
}

export interface AuthPayload {
  token: string;
  user: AuthUser;
}

export interface RegisterPayload extends AuthPayload {
  message: string;
}

export interface SessionPayload {
  user: AuthUser;
}

export interface SuccessResponse {
  success: true;
}

function parseAuthUser(value: unknown, path: string): AuthUser {
  const record = asRecord(value, path);
  const role = readNonEmptyString(record, 'role', path);
  if (role !== 'admin' && role !== 'user') {
    fail(`${path}.role`, 'must be one of: admin, user');
  }

  return {
    id: readNonEmptyString(record, 'id', path),
    name: readNonEmptyString(record, 'name', path),
    role,
  };
}

export function parseAuthPayload(value: unknown): AuthPayload {
  const record = asRecord(value, 'response');
  return {
    token: readNonEmptyString(record, 'token', 'response'),
    user: parseAuthUser(readObject(record, 'user', 'response'), 'response.user'),
  };
}

export function parseRegisterPayload(value: unknown): RegisterPayload {
  const payload = parseAuthPayload(value);
  const record = asRecord(value, 'response');

  return {
    ...payload,
    message: readString(record, 'message', 'response'),
  };
}

export function parseSessionPayload(value: unknown): SessionPayload {
  const record = asRecord(value, 'response');
  return {
    user: parseAuthUser(readObject(record, 'user', 'response'), 'response.user'),
  };
}

export function parseSuccessResponse(value: unknown): SuccessResponse {
  const record = asRecord(value, 'response');
  const success = readBoolean(record, 'success', 'response');
  if (!success) {
    fail('response.success', 'must be true');
  }
  return { success: true };
}

export interface AssetFolder {
  id: string;
  userId: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface AssetCharacterAppearance {
  id: string;
  appearanceIndex: number;
  changeReason: string;
  description: string | null;
  imageUrl: string | null;
  imageUrls: string[];
  selectedIndex: number | null;
}

export interface AssetCharacter {
  id: string;
  folderId: string | null;
  name: string;
  voiceId: string | null;
  voiceType: string | null;
  customVoiceUrl: string | null;
  globalVoiceId: string | null;
  appearances: AssetCharacterAppearance[];
}

export interface AssetLocationImage {
  id: string;
  imageIndex: number;
  imageUrl: string | null;
  description: string | null;
  isSelected: boolean;
}

export interface AssetLocation {
  id: string;
  folderId: string | null;
  name: string;
  summary: string | null;
  images: AssetLocationImage[];
}

export interface AssetVoice {
  id: string;
  folderId: string | null;
  name: string;
  description: string | null;
  voiceId: string | null;
  voiceType: string;
  customVoiceUrl: string | null;
  voicePrompt: string | null;
  language: string;
  gender: string | null;
}

export interface AssetFolderListResponse {
  folders: AssetFolder[];
}

export interface AssetFolderMutationResponse {
  success: true;
  folder: AssetFolder;
}

export interface AssetCharacterListResponse {
  characters: AssetCharacter[];
}

export interface AssetCharacterMutationResponse {
  success: true;
  character: AssetCharacter;
}

export interface AssetLocationListResponse {
  locations: AssetLocation[];
}

export interface AssetLocationMutationResponse {
  success: true;
  location: AssetLocation;
}

export interface AssetVoiceListResponse {
  voices: AssetVoice[];
}

export interface AssetVoiceMutationResponse {
  success: true;
  voice: AssetVoice;
}

function parseAssetFolder(value: unknown, path: string): AssetFolder {
  const record = asRecord(value, path);
  return {
    id: readNonEmptyString(record, 'id', path),
    userId: readNonEmptyString(record, 'userId', path),
    name: readString(record, 'name', path),
    createdAt: readNonEmptyString(record, 'createdAt', path),
    updatedAt: readNonEmptyString(record, 'updatedAt', path),
  };
}

function parseAssetCharacterAppearance(
  value: unknown,
  path: string,
): AssetCharacterAppearance {
  const record = asRecord(value, path);
  return {
    id: readNonEmptyString(record, 'id', path),
    appearanceIndex: readInteger(record, 'appearanceIndex', path),
    changeReason: readString(record, 'changeReason', path),
    description: readNullableString(record, 'description', path),
    imageUrl: readNullableString(record, 'imageUrl', path),
    imageUrls: readStringArray(record, 'imageUrls', path),
    selectedIndex: readNullableInteger(record, 'selectedIndex', path),
  };
}

function parseAssetCharacter(value: unknown, path: string): AssetCharacter {
  const record = asRecord(value, path);
  const appearanceValues = readArray(record, 'appearances', path);

  return {
    id: readNonEmptyString(record, 'id', path),
    folderId: readNullableString(record, 'folderId', path),
    name: readString(record, 'name', path),
    voiceId: readNullableString(record, 'voiceId', path),
    voiceType: readNullableString(record, 'voiceType', path),
    customVoiceUrl: readNullableString(record, 'customVoiceUrl', path),
    globalVoiceId: readNullableString(record, 'globalVoiceId', path),
    appearances: appearanceValues.map((item, index) =>
      parseAssetCharacterAppearance(item, `${path}.appearances[${index}]`),
    ),
  };
}

function parseAssetLocationImage(value: unknown, path: string): AssetLocationImage {
  const record = asRecord(value, path);
  return {
    id: readNonEmptyString(record, 'id', path),
    imageIndex: readInteger(record, 'imageIndex', path),
    imageUrl: readNullableString(record, 'imageUrl', path),
    description: readNullableString(record, 'description', path),
    isSelected: readBoolean(record, 'isSelected', path),
  };
}

function parseAssetLocation(value: unknown, path: string): AssetLocation {
  const record = asRecord(value, path);
  const imageValues = readArray(record, 'images', path);

  return {
    id: readNonEmptyString(record, 'id', path),
    folderId: readNullableString(record, 'folderId', path),
    name: readString(record, 'name', path),
    summary: readNullableString(record, 'summary', path),
    images: imageValues.map((item, index) =>
      parseAssetLocationImage(item, `${path}.images[${index}]`),
    ),
  };
}

function parseAssetVoice(value: unknown, path: string): AssetVoice {
  const record = asRecord(value, path);

  return {
    id: readNonEmptyString(record, 'id', path),
    folderId: readNullableString(record, 'folderId', path),
    name: readString(record, 'name', path),
    description: readNullableString(record, 'description', path),
    voiceId: readNullableString(record, 'voiceId', path),
    voiceType: readString(record, 'voiceType', path),
    customVoiceUrl: readNullableString(record, 'customVoiceUrl', path),
    voicePrompt: readNullableString(record, 'voicePrompt', path),
    language: readString(record, 'language', path),
    gender: readNullableString(record, 'gender', path),
  };
}

export function parseAssetFolderListResponse(
  value: unknown,
): AssetFolderListResponse {
  const record = asRecord(value, 'response');
  const folders = readArray(record, 'folders', 'response').map((item, index) =>
    parseAssetFolder(item, `response.folders[${index}]`),
  );

  return { folders };
}

export function parseAssetFolderMutationResponse(
  value: unknown,
): AssetFolderMutationResponse {
  const record = asRecord(value, 'response');
  parseSuccessResponse(value);

  return {
    success: true,
    folder: parseAssetFolder(readObject(record, 'folder', 'response'), 'response.folder'),
  };
}

export function parseAssetCharacterListResponse(
  value: unknown,
): AssetCharacterListResponse {
  const record = asRecord(value, 'response');
  const characters = readArray(record, 'characters', 'response').map((item, index) =>
    parseAssetCharacter(item, `response.characters[${index}]`),
  );

  return { characters };
}

export function parseAssetCharacterMutationResponse(
  value: unknown,
): AssetCharacterMutationResponse {
  const record = asRecord(value, 'response');
  parseSuccessResponse(value);

  return {
    success: true,
    character: parseAssetCharacter(
      readObject(record, 'character', 'response'),
      'response.character',
    ),
  };
}

export function parseAssetLocationListResponse(
  value: unknown,
): AssetLocationListResponse {
  const record = asRecord(value, 'response');
  const locations = readArray(record, 'locations', 'response').map((item, index) =>
    parseAssetLocation(item, `response.locations[${index}]`),
  );

  return { locations };
}

export function parseAssetLocationMutationResponse(
  value: unknown,
): AssetLocationMutationResponse {
  const record = asRecord(value, 'response');
  parseSuccessResponse(value);

  return {
    success: true,
    location: parseAssetLocation(
      readObject(record, 'location', 'response'),
      'response.location',
    ),
  };
}

export function parseAssetVoiceListResponse(
  value: unknown,
): AssetVoiceListResponse {
  const record = asRecord(value, 'response');
  const voices = readArray(record, 'voices', 'response').map((item, index) =>
    parseAssetVoice(item, `response.voices[${index}]`),
  );

  return { voices };
}

export function parseAssetVoiceMutationResponse(
  value: unknown,
): AssetVoiceMutationResponse {
  const record = asRecord(value, 'response');
  parseSuccessResponse(value);

  return {
    success: true,
    voice: parseAssetVoice(readObject(record, 'voice', 'response'), 'response.voice'),
  };
}

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

function parseTaskEvent(
  value: unknown,
  expectedType: TaskLifecycleEvent['type'] | TaskStreamEvent['type'],
): TaskLifecycleEvent | TaskStreamEvent {
  const record = asRecord(value, 'event');
  const type = readNonEmptyString(record, 'type', 'event');
  if (type !== expectedType) {
    fail('event.type', `must equal ${expectedType}`);
  }

  return {
    id: readNonEmptyString(record, 'id', 'event'),
    type,
    taskId: readNonEmptyString(record, 'taskId', 'event'),
    projectId: readNonEmptyString(record, 'projectId', 'event'),
    userId: readNonEmptyString(record, 'userId', 'event'),
    eventType: readNonEmptyString(record, 'eventType', 'event'),
    taskType: readOptionalString(record, 'taskType', 'event'),
    targetType: readOptionalString(record, 'targetType', 'event'),
    targetId: readOptionalString(record, 'targetId', 'event'),
    episodeId: readOptionalString(record, 'episodeId', 'event'),
    payload: readObject(record, 'payload', 'event'),
    ts: readNonEmptyString(record, 'ts', 'event'),
  };
}

export function parseTaskLifecycleEvent(
  value: unknown,
): TaskLifecycleEvent {
  return parseTaskEvent(value, 'task.lifecycle') as TaskLifecycleEvent;
}

export function parseTaskStreamEvent(value: unknown): TaskStreamEvent {
  return parseTaskEvent(value, 'task.stream') as TaskStreamEvent;
}

export function parseHeartbeatPayload(value: unknown): HeartbeatPayload {
  const record = asRecord(value, 'event');
  return {
    ts: readNonEmptyString(record, 'ts', 'event'),
  };
}
