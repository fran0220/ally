#!/usr/bin/env node

import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_BASE_URL = 'http://127.0.0.1:3001'
const DEFAULT_TIMEOUT_MS = 30000
const PHASE_SEQUENCE = [
  'auth-flow',
  'project-lifecycle',
  'asset-hub-crud',
  'novel-promotion',
  'user-config',
  'admin-billing',
]

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {
    clean: false,
    dryRun: false,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

    if (token === '--clean') {
      args.clean = true
      continue
    }

    if (token === '--dry-run') {
      args.dryRun = true
      continue
    }

    const key = token.slice(2)
    const value = argv[i + 1]
    assert(value && !value.startsWith('--'), `Missing value for --${key}`)
    args[key] = value
    i += 1
  }

  return args
}

function normalizeBaseUrl(raw, name) {
  assert(typeof raw === 'string' && raw.trim().length > 0, `${name} is required`)
  const normalized = raw.trim()
  assert(/^https?:\/\//.test(normalized), `${name} must start with http:// or https://`)
  return normalized.endsWith('/') ? normalized : `${normalized}/`
}

function parseOptionalInt(raw, name, fallback) {
  if (raw === undefined) {
    return fallback
  }
  const parsed = Number(raw)
  assert(Number.isInteger(parsed) && parsed > 0, `${name} must be a positive integer`)
  return parsed
}

function safeJsonParse(raw) {
  try {
    return { ok: true, value: JSON.parse(raw) }
  } catch {
    return { ok: false, value: null }
  }
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function markdownCell(value) {
  return String(value ?? '')
    .replace(/\|/g, '\\|')
    .replace(/\n/g, ' ')
}

function createIdentity() {
  const ts = Date.now()
  const nonce = crypto.randomUUID().slice(0, 8)
  const username = `e2e-run-all-${ts}-${nonce}@test.local`
  return {
    username,
    password: 'Test123456',
    projectName: `E2E Run All ${ts}`,
  }
}

async function requestJson({ baseUrl, method, endpointPath, token, body, timeoutMs }) {
  const targetUrl = new URL(endpointPath, baseUrl).toString()
  const headers = {
    Accept: 'application/json',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  let payload
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
    payload = JSON.stringify(body)
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method,
      headers,
      body: payload,
      signal: controller.signal,
    })

    const rawBody = await response.text()
    const parsed = safeJsonParse(rawBody)

    return {
      status: response.status,
      url: targetUrl,
      rawBody,
      json: parsed.ok ? parsed.value : null,
      parseError: parsed.ok ? null : 'response is not valid JSON',
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`request failed ${method} ${endpointPath}: ${message}`)
  } finally {
    clearTimeout(timer)
  }
}

function expectStatus(response, expectedStatus, context) {
  assert(
    response.status === expectedStatus,
    `${context}: expected status ${expectedStatus}, got ${response.status}, body=${response.rawBody}`,
  )
}

function expectStatusOneOf(response, expectedStatuses, context) {
  assert(
    expectedStatuses.includes(response.status),
    `${context}: expected status ${expectedStatuses.join('/')} got ${response.status}, body=${response.rawBody}`,
  )
}

function expectJsonObject(response, context) {
  assert(response.parseError === null, `${context}: ${response.parseError}`)
  assert(isObject(response.json), `${context}: response body must be a JSON object`)
  return response.json
}

function ensureToken(context) {
  assert(
    typeof context.token === 'string' && context.token.trim().length > 0,
    'auth token is missing; auth-flow phase did not complete successfully',
  )
  return context.token
}

function ensureProjectId(context) {
  assert(
    typeof context.projectId === 'string' && context.projectId.trim().length > 0,
    'project id is missing; project-lifecycle phase did not complete successfully',
  )
  return context.projectId
}

function createPhase(id) {
  return {
    id,
    status: 'PASS',
    startedAt: new Date().toISOString(),
    endedAt: null,
    durationMs: 0,
    steps: [],
    error: null,
  }
}

async function runStep(phase, metadata, executor) {
  const startedAt = Date.now()

  try {
    const result = await executor()
    const row = {
      ...metadata,
      result: 'PASS',
      httpStatus: result.httpStatus ?? '',
      durationMs: Date.now() - startedAt,
      note: result.note ?? '',
    }
    phase.steps.push(row)
    return {
      ok: true,
      value: result.value ?? null,
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    phase.status = 'FAIL'
    phase.steps.push({
      ...metadata,
      result: 'FAIL',
      httpStatus: '',
      durationMs: Date.now() - startedAt,
      note: message,
    })
    return {
      ok: false,
      value: null,
    }
  }
}

function finalizePhase(phase, startedAtMs) {
  phase.endedAt = new Date().toISOString()
  phase.durationMs = Date.now() - startedAtMs
  if (phase.steps.some((step) => step.result === 'FAIL')) {
    phase.status = 'FAIL'
  }
}

async function runAuthFlow(phase, context) {
  const registerStep = await runStep(
    phase,
    {
      id: 'register-user',
      method: 'POST',
      path: '/api/auth/register',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/auth/register',
        body: {
          name: context.identity.username,
          password: context.identity.password,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [200, 201], 'register user')
      const body = expectJsonObject(response, 'register user')
      assert(typeof body.token === 'string' && body.token.trim().length > 0, 'register user: token is missing')
      assert(isObject(body.user), 'register user: user is missing')
      assert(typeof body.user.id === 'string' && body.user.id.trim().length > 0, 'register user: user.id is missing')

      context.token = body.token
      context.userId = body.user.id

      return {
        httpStatus: response.status,
        note: `userId=${context.userId}`,
      }
    },
  )

  if (!registerStep.ok) {
    return
  }

  const loginStep = await runStep(
    phase,
    {
      id: 'login-user',
      method: 'POST',
      path: '/api/auth/login',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/auth/login',
        body: {
          username: context.identity.username,
          password: context.identity.password,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'login user')
      const body = expectJsonObject(response, 'login user')
      assert(typeof body.token === 'string' && body.token.trim().length > 0, 'login user: token is missing')
      assert(isObject(body.user), 'login user: user is missing')

      context.token = body.token

      return {
        httpStatus: response.status,
        note: `role=${body.user.role ?? 'unknown'}`,
      }
    },
  )

  if (!loginStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'refresh-token',
      method: 'POST',
      path: '/api/auth/refresh',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/auth/refresh',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'refresh token')
      const body = expectJsonObject(response, 'refresh token')
      assert(typeof body.token === 'string' && body.token.trim().length > 0, 'refresh token: token is missing')
      context.token = body.token

      return {
        httpStatus: response.status,
        note: 'token refreshed',
      }
    },
  )
}

async function runProjectLifecycle(phase, context) {
  const createStep = await runStep(
    phase,
    {
      id: 'create-project',
      method: 'POST',
      path: '/api/projects',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/projects',
        token: ensureToken(context),
        body: {
          name: context.identity.projectName,
          description: 'created by e2e-run-all',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create project')
      const body = expectJsonObject(response, 'create project')
      assert(isObject(body.project), 'create project: project payload missing')
      assert(typeof body.project.id === 'string' && body.project.id.trim().length > 0, 'create project: project.id missing')
      context.projectId = body.project.id

      return {
        httpStatus: response.status,
        note: `projectId=${context.projectId}`,
      }
    },
  )

  if (!createStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'list-projects',
      method: 'GET',
      path: '/api/projects?page=1&pageSize=20',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/projects?page=1&pageSize=20',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list projects')
      const body = expectJsonObject(response, 'list projects')
      assert(Array.isArray(body.projects), 'list projects: projects must be an array')
      const found = body.projects.some((item) => isObject(item) && item.id === context.projectId)
      assert(found, `list projects: project ${context.projectId} not found`)

      return {
        httpStatus: response.status,
        note: `listed=${body.projects.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-project',
      method: 'GET',
      path: `/api/projects/${context.projectId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(ensureProjectId(context))}`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get project')
      const body = expectJsonObject(response, 'get project')
      assert(isObject(body.project), 'get project: project payload missing')
      assert(body.project.id === context.projectId, 'get project: id mismatch')

      return {
        httpStatus: response.status,
        note: `name=${body.project.name}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-project-data',
      method: 'GET',
      path: `/api/projects/${context.projectId}/data`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(ensureProjectId(context))}/data`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get project data')
      const body = expectJsonObject(response, 'get project data')
      assert(isObject(body.project), 'get project data: project payload missing')
      assert(isObject(body.project.novelPromotionData), 'get project data: novelPromotionData missing')

      return {
        httpStatus: response.status,
        note: `novelId=${body.project.novelPromotionData.id}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-project-assets',
      method: 'GET',
      path: `/api/projects/${context.projectId}/assets`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(ensureProjectId(context))}/assets`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get project assets')
      const body = expectJsonObject(response, 'get project assets')
      assert(Array.isArray(body.characters), 'get project assets: characters must be an array')
      assert(Array.isArray(body.locations), 'get project assets: locations must be an array')

      return {
        httpStatus: response.status,
        note: `characters=${body.characters.length}, locations=${body.locations.length}`,
      }
    },
  )
}

async function runAssetHubCrud(phase, context) {
  const characterCreate = await runStep(
    phase,
    {
      id: 'create-asset-character',
      method: 'POST',
      path: '/api/asset-hub/characters',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/asset-hub/characters',
        token: ensureToken(context),
        body: {
          name: `E2E Character ${Date.now()}`,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create asset character')
      const body = expectJsonObject(response, 'create asset character')
      assert(isObject(body.character), 'create asset character: character payload missing')
      assert(typeof body.character.id === 'string' && body.character.id.trim().length > 0, 'create asset character: id missing')
      context.assetCharacterIds.push(body.character.id)

      return {
        httpStatus: response.status,
        note: `characterId=${body.character.id}`,
        value: {
          characterId: body.character.id,
        },
      }
    },
  )

  if (characterCreate.ok) {
    await runStep(
      phase,
      {
        id: 'update-asset-character',
        method: 'PATCH',
        path: `/api/asset-hub/characters/${characterCreate.value.characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterCreate.value.characterId)}`,
          token: ensureToken(context),
          body: {
            name: 'E2E Character Updated',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'update asset character')
        const body = expectJsonObject(response, 'update asset character')
        assert(isObject(body.character), 'update asset character: character payload missing')

        return {
          httpStatus: response.status,
          note: `updated=${body.character.id}`,
        }
      },
    )
  }

  const locationCreate = await runStep(
    phase,
    {
      id: 'create-asset-location',
      method: 'POST',
      path: '/api/asset-hub/locations',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/asset-hub/locations',
        token: ensureToken(context),
        body: {
          name: `E2E Location ${Date.now()}`,
          summary: 'e2e location summary',
          description: 'e2e location description',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create asset location')
      const body = expectJsonObject(response, 'create asset location')
      assert(isObject(body.location), 'create asset location: location payload missing')
      assert(typeof body.location.id === 'string' && body.location.id.trim().length > 0, 'create asset location: id missing')
      context.assetLocationIds.push(body.location.id)

      return {
        httpStatus: response.status,
        note: `locationId=${body.location.id}`,
        value: {
          locationId: body.location.id,
        },
      }
    },
  )

  const voiceCreate = await runStep(
    phase,
    {
      id: 'create-asset-voice',
      method: 'POST',
      path: '/api/asset-hub/voices',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/asset-hub/voices',
        token: ensureToken(context),
        body: {
          name: `E2E Voice ${Date.now()}`,
          description: 'e2e voice description',
          voiceType: 'qwen-designed',
          language: 'zh',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create asset voice')
      const body = expectJsonObject(response, 'create asset voice')
      assert(isObject(body.voice), 'create asset voice: voice payload missing')
      assert(typeof body.voice.id === 'string' && body.voice.id.trim().length > 0, 'create asset voice: id missing')
      context.assetVoiceIds.push(body.voice.id)

      return {
        httpStatus: response.status,
        note: `voiceId=${body.voice.id}`,
        value: {
          voiceId: body.voice.id,
        },
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'list-asset-locations',
      method: 'GET',
      path: '/api/asset-hub/locations',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/asset-hub/locations',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list asset locations')
      const body = expectJsonObject(response, 'list asset locations')
      assert(Array.isArray(body.locations), 'list asset locations: locations must be an array')

      const exists =
        locationCreate.ok &&
        body.locations.some((item) => isObject(item) && item.id === locationCreate.value.locationId)

      return {
        httpStatus: response.status,
        note: `count=${body.locations.length}${exists ? ' include-new' : ''}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'list-asset-voices',
      method: 'GET',
      path: '/api/asset-hub/voices',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/asset-hub/voices',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list asset voices')
      const body = expectJsonObject(response, 'list asset voices')
      assert(Array.isArray(body.voices), 'list asset voices: voices must be an array')

      const exists =
        voiceCreate.ok && body.voices.some((item) => isObject(item) && item.id === voiceCreate.value.voiceId)

      return {
        httpStatus: response.status,
        note: `count=${body.voices.length}${exists ? ' include-new' : ''}`,
      }
    },
  )

  if (characterCreate.ok) {
    const deleteStep = await runStep(
      phase,
      {
        id: 'delete-asset-character',
        method: 'DELETE',
        path: `/api/asset-hub/characters/${characterCreate.value.characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterCreate.value.characterId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'delete asset character')
        const body = expectJsonObject(response, 'delete asset character')
        assert(body.success === true, 'delete asset character: success must be true')

        return {
          httpStatus: response.status,
          note: 'deleted',
        }
      },
    )

    if (deleteStep.ok) {
      context.assetCharacterIds = context.assetCharacterIds.filter(
        (id) => id !== characterCreate.value.characterId,
      )
    }
  }
}

async function runNovelPromotion(phase, context) {
  const projectId = ensureProjectId(context)

  await runStep(
    phase,
    {
      id: 'get-novel-root',
      method: 'GET',
      path: `/api/novel-promotion/${projectId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get novel root')
      const body = expectJsonObject(response, 'get novel root')
      const novelData = body.project?.novelPromotionData ?? body
      assert(Array.isArray(novelData.episodes), 'get novel root: episodes must be an array')

      return {
        httpStatus: response.status,
        note: `episodes=${novelData.episodes.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-novel-episodes',
      method: 'GET',
      path: `/api/novel-promotion/${projectId}/episodes`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get novel episodes')
      const body = expectJsonObject(response, 'get novel episodes')
      assert(Array.isArray(body.episodes), 'get novel episodes: episodes must be an array')

      return {
        httpStatus: response.status,
        note: `episodes=${body.episodes.length}`,
      }
    },
  )

  const createEpisode = await runStep(
    phase,
    {
      id: 'create-novel-episode',
      method: 'POST',
      path: `/api/novel-promotion/${projectId}/episodes`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes`,
        token: ensureToken(context),
        body: {
          name: `E2E Episode ${Date.now()}`,
          novelText: 'Generated by e2e-run-all for contract coverage.',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create novel episode')
      const body = expectJsonObject(response, 'create novel episode')
      assert(isObject(body.episode), 'create novel episode: episode payload missing')
      assert(typeof body.episode.id === 'string' && body.episode.id.trim().length > 0, 'create novel episode: id missing')
      context.novelEpisodeId = body.episode.id

      return {
        httpStatus: response.status,
        note: `episodeId=${context.novelEpisodeId}`,
        value: {
          episodeId: context.novelEpisodeId,
        },
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-novel-characters',
      method: 'GET',
      path: `/api/novel-promotion/${projectId}/characters`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/characters`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get novel characters')
      const body = expectJsonObject(response, 'get novel characters')
      assert(Array.isArray(body.characters), 'get novel characters: characters must be an array')

      return {
        httpStatus: response.status,
        note: `characters=${body.characters.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-novel-locations',
      method: 'GET',
      path: `/api/novel-promotion/${projectId}/locations`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/locations`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get novel locations')
      const body = expectJsonObject(response, 'get novel locations')
      assert(Array.isArray(body.locations), 'get novel locations: locations must be an array')

      return {
        httpStatus: response.status,
        note: `locations=${body.locations.length}`,
      }
    },
  )

  const createCharacter = await runStep(
    phase,
    {
      id: 'create-novel-character',
      method: 'POST',
      path: `/api/novel-promotion/${projectId}/characters`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/characters`,
        token: ensureToken(context),
        body: {
          name: `E2E Novel Character ${Date.now()}`,
          description: 'e2e novel character description',
          meta: {
            locale: 'zh-CN',
          },
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create novel character')
      const body = expectJsonObject(response, 'create novel character')
      assert(isObject(body.character), 'create novel character: character payload missing')
      assert(typeof body.character.id === 'string' && body.character.id.trim().length > 0, 'create novel character: id missing')
      context.novelCharacterId = body.character.id

      return {
        httpStatus: response.status,
        note: `characterId=${context.novelCharacterId}`,
      }
    },
  )

  if (!createEpisode.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'get-novel-storyboards',
      method: 'GET',
      path: `/api/novel-promotion/${projectId}/storyboards?episodeId=${createEpisode.value.episodeId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboards?episodeId=${encodeURIComponent(createEpisode.value.episodeId)}`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get novel storyboards')
      const body = expectJsonObject(response, 'get novel storyboards')
      assert(Array.isArray(body.storyboards), 'get novel storyboards: storyboards must be an array')

      return {
        httpStatus: response.status,
        note: `storyboards=${body.storyboards.length}`,
      }
    },
  )

  if (!createCharacter.ok) {
    return
  }
}

async function runUserConfig(phase, context) {
  await runStep(
    phase,
    {
      id: 'get-user-models',
      method: 'GET',
      path: '/api/user/models',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/models',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get user models')
      const body = expectJsonObject(response, 'get user models')
      assert(Array.isArray(body.llm), 'get user models: llm must be an array')

      return {
        httpStatus: response.status,
        note: `llm=${body.llm.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-user-api-config',
      method: 'GET',
      path: '/api/user/api-config',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/api-config',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get user api config')
      const body = expectJsonObject(response, 'get user api config')
      assert(Array.isArray(body.providers), 'get user api config: providers must be an array')
      assert(Array.isArray(body.models), 'get user api config: models must be an array')

      return {
        httpStatus: response.status,
        note: `providers=${body.providers.length}, models=${body.models.length}`,
      }
    },
  )

  const preferenceStep = await runStep(
    phase,
    {
      id: 'get-user-preference',
      method: 'GET',
      path: '/api/user-preference',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user-preference',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'get user preference')
      const body = expectJsonObject(response, 'get user preference')
      assert(isObject(body.preference), 'get user preference: preference payload missing')

      return {
        httpStatus: response.status,
        note: `artStyle=${body.preference.artStyle ?? 'unknown'}`,
        value: {
          artStyle: typeof body.preference.artStyle === 'string' ? body.preference.artStyle : 'american-comic',
        },
      }
    },
  )

  if (preferenceStep.ok) {
    await runStep(
      phase,
      {
        id: 'patch-user-preference',
        method: 'PATCH',
        path: '/api/user-preference',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: '/api/user-preference',
          token: ensureToken(context),
          body: {
            artStyle: preferenceStep.value.artStyle,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch user preference')
        const body = expectJsonObject(response, 'patch user preference')
        assert(isObject(body.preference), 'patch user preference: preference payload missing')

        return {
          httpStatus: response.status,
          note: `artStyle=${body.preference.artStyle ?? 'unknown'}`,
        }
      },
    )
  }

  await runStep(
    phase,
    {
      id: 'test-user-api-config-invalid',
      method: 'POST',
      path: '/api/user/api-config/test-connection',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/user/api-config/test-connection',
        token: ensureToken(context),
        body: {},
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 400, 'test user api config invalid')
      expectJsonObject(response, 'test user api config invalid')

      return {
        httpStatus: response.status,
        note: 'empty payload rejected as expected',
      }
    },
  )
}

async function runAdminBilling(phase, context) {
  await runStep(
    phase,
    {
      id: 'admin-ai-config-non-admin',
      method: 'GET',
      path: '/api/admin/ai-config',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/admin/ai-config',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [401, 403], 'admin ai-config non-admin')
      expectJsonObject(response, 'admin ai-config non-admin')

      return {
        httpStatus: response.status,
        note: 'non-admin rejected as expected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'billing-balance',
      method: 'GET',
      path: '/api/user/balance',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/balance',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'billing balance')
      const body = expectJsonObject(response, 'billing balance')
      assert(body.currency !== undefined, 'billing balance: currency is missing')
      assert(body.balance !== undefined, 'billing balance: balance is missing')

      return {
        httpStatus: response.status,
        note: `currency=${body.currency}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'billing-costs',
      method: 'GET',
      path: '/api/user/costs',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/costs',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'billing costs')
      const body = expectJsonObject(response, 'billing costs')
      assert(Array.isArray(body.byProject), 'billing costs: byProject must be an array')

      return {
        httpStatus: response.status,
        note: `projects=${body.byProject.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'billing-cost-details',
      method: 'GET',
      path: '/api/user/costs/details?page=1&pageSize=10',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/costs/details?page=1&pageSize=10',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'billing cost details')
      const body = expectJsonObject(response, 'billing cost details')
      assert(Array.isArray(body.records), 'billing cost details: records must be an array')

      return {
        httpStatus: response.status,
        note: `records=${body.records.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'billing-transactions',
      method: 'GET',
      path: '/api/user/transactions?page=1&pageSize=10',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/user/transactions?page=1&pageSize=10',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'billing transactions')
      const body = expectJsonObject(response, 'billing transactions')
      assert(Array.isArray(body.transactions), 'billing transactions: transactions must be an array')

      return {
        httpStatus: response.status,
        note: `transactions=${body.transactions.length}`,
      }
    },
  )
}

async function runCleanupStep(cleanup, metadata, executor) {
  const startedAt = Date.now()

  try {
    const result = await executor()
    cleanup.steps.push({
      ...metadata,
      result: 'PASS',
      httpStatus: result.httpStatus ?? '',
      durationMs: Date.now() - startedAt,
      note: result.note ?? '',
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    cleanup.steps.push({
      ...metadata,
      result: 'FAIL',
      httpStatus: '',
      durationMs: Date.now() - startedAt,
      note: message,
    })
  }
}

async function runCleanup(report, context) {
  const cleanup = {
    requested: context.clean,
    startedAt: new Date().toISOString(),
    endedAt: null,
    steps: [],
  }
  report.cleanup = cleanup

  if (!context.clean) {
    cleanup.endedAt = new Date().toISOString()
    return
  }

  if (context.token && context.projectId && context.novelCharacterId) {
    await runCleanupStep(
      cleanup,
      {
        id: 'delete-novel-character',
        method: 'DELETE',
        path: `/api/novel-promotion/${context.projectId}/characters`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(context.projectId)}/characters`,
          token: context.token,
          body: {
            characterId: context.novelCharacterId,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete novel character')

        return {
          httpStatus: response.status,
          note: `characterId=${context.novelCharacterId}`,
        }
      },
    )
  }

  for (const characterId of context.assetCharacterIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-asset-character-${characterId}`,
        method: 'DELETE',
        path: `/api/asset-hub/characters/${characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete asset character')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  for (const locationId of context.assetLocationIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-asset-location-${locationId}`,
        method: 'DELETE',
        path: `/api/asset-hub/locations/${locationId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/locations/${encodeURIComponent(locationId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete asset location')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  for (const voiceId of context.assetVoiceIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-asset-voice-${voiceId}`,
        method: 'DELETE',
        path: `/api/asset-hub/voices/${voiceId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/voices/${encodeURIComponent(voiceId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete asset voice')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  if (context.token && context.projectId) {
    await runCleanupStep(
      cleanup,
      {
        id: 'delete-project',
        method: 'DELETE',
        path: `/api/projects/${context.projectId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/projects/${encodeURIComponent(context.projectId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete project')

        return {
          httpStatus: response.status,
          note: `projectId=${context.projectId}`,
        }
      },
    )
  }

  cleanup.endedAt = new Date().toISOString()
}

function buildSummary(report) {
  const phaseTotal = report.phases.length
  const phasePass = report.phases.filter((phase) => phase.status === 'PASS').length
  const phaseFail = phaseTotal - phasePass

  const allSteps = report.phases.flatMap((phase) => phase.steps)
  const stepTotal = allSteps.length
  const stepPass = allSteps.filter((step) => step.result === 'PASS').length
  const stepFail = stepTotal - stepPass

  const cleanupSteps = report.cleanup?.steps || []
  const cleanupPass = cleanupSteps.filter((step) => step.result === 'PASS').length
  const cleanupFail = cleanupSteps.length - cleanupPass

  return {
    phaseTotal,
    phasePass,
    phaseFail,
    stepTotal,
    stepPass,
    stepFail,
    cleanupTotal: cleanupSteps.length,
    cleanupPass,
    cleanupFail,
  }
}

function renderMarkdown(report) {
  const summary = buildSummary(report)
  const lines = []

  lines.push('# E2E Run All Summary')
  lines.push('')
  lines.push(`- Generated at: ${report.generatedAt}`)
  lines.push(`- Base URL: ${report.baseUrl}`)
  lines.push(`- Clean requested: ${report.cleanRequested}`)
  lines.push(`- E2E user: ${report.identity.username}`)
  lines.push(`- User ID: ${report.userId || 'N/A'}`)
  lines.push(`- Project ID: ${report.projectId || 'N/A'}`)
  lines.push(`- Phases: ${summary.phaseTotal} (pass=${summary.phasePass}, fail=${summary.phaseFail})`)
  lines.push(`- Steps: ${summary.stepTotal} (pass=${summary.stepPass}, fail=${summary.stepFail})`)
  lines.push(`- Cleanup steps: ${summary.cleanupTotal} (pass=${summary.cleanupPass}, fail=${summary.cleanupFail})`)
  lines.push('')
  lines.push('## Phases')
  lines.push('')
  lines.push('| Phase | Result | Steps | Pass | Fail | Duration(ms) | Error |')
  lines.push('|---|---|---|---|---|---|---|')

  for (const phase of report.phases) {
    const passCount = phase.steps.filter((step) => step.result === 'PASS').length
    const failCount = phase.steps.length - passCount
    lines.push(
      `| ${phase.id} | ${phase.status} | ${phase.steps.length} | ${passCount} | ${failCount} | ${phase.durationMs} | ${markdownCell(phase.error || '')} |`,
    )
  }

  for (const phase of report.phases) {
    lines.push('')
    lines.push(`## ${phase.id}`)
    lines.push('')
    lines.push('| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |')
    lines.push('|---|---|---|---|---|---|---|')
    for (const step of phase.steps) {
      lines.push(
        `| ${markdownCell(step.id)} | ${step.result} | ${step.method} | ${markdownCell(step.path)} | ${markdownCell(step.httpStatus)} | ${step.durationMs} | ${markdownCell(step.note)} |`,
      )
    }
  }

  lines.push('')
  lines.push('## Cleanup')
  lines.push('')
  if (!report.cleanup || report.cleanup.steps.length === 0) {
    lines.push('- no cleanup steps executed')
  } else {
    lines.push('| Step | Result | Method | Path | HTTP Status | Duration(ms) | Note |')
    lines.push('|---|---|---|---|---|---|---|')
    for (const step of report.cleanup.steps) {
      lines.push(
        `| ${markdownCell(step.id)} | ${step.result} | ${step.method} | ${markdownCell(step.path)} | ${markdownCell(step.httpStatus)} | ${step.durationMs} | ${markdownCell(step.note)} |`,
      )
    }
  }

  const failures = []
  for (const phase of report.phases) {
    for (const step of phase.steps) {
      if (step.result === 'FAIL') {
        failures.push(`${phase.id}/${step.id}: ${step.note}`)
      }
    }
  }
  if (report.cleanup) {
    for (const step of report.cleanup.steps) {
      if (step.result === 'FAIL') {
        failures.push(`cleanup/${step.id}: ${step.note}`)
      }
    }
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')
  if (failures.length === 0) {
    lines.push('- none')
  } else {
    for (const line of failures) {
      lines.push(`- ${line}`)
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReports(outDir, report) {
  fs.mkdirSync(outDir, { recursive: true })
  const jsonPath = path.join(outDir, 'e2e-summary.json')
  const mdPath = path.join(outDir, 'e2e-summary.md')

  const payload = {
    ...report,
    summary: buildSummary(report),
  }

  fs.writeFileSync(jsonPath, JSON.stringify(payload, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdown(payload), 'utf8')

  return { jsonPath, mdPath, summary: payload.summary }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'] || DEFAULT_BASE_URL, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms', DEFAULT_TIMEOUT_MS)

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] timeout-ms: ${timeoutMs}`)
    console.log(`[dry-run] clean: ${args.clean}`)
    console.log(`[dry-run] report: ${path.join(outDir, 'e2e-summary.md')}`)
    console.log(`[dry-run] phases: ${PHASE_SEQUENCE.join(' -> ')}`)
    return
  }

  const identity = createIdentity()
  const context = {
    baseUrl,
    timeoutMs,
    clean: args.clean,
    identity,
    token: null,
    userId: null,
    projectId: null,
    novelEpisodeId: null,
    novelCharacterId: null,
    assetCharacterIds: [],
    assetLocationIds: [],
    assetVoiceIds: [],
  }

  const report = {
    generatedAt: new Date().toISOString(),
    baseUrl,
    cleanRequested: args.clean,
    identity: {
      username: identity.username,
    },
    userId: null,
    projectId: null,
    phases: [],
    cleanup: null,
  }

  const phaseHandlers = {
    'auth-flow': runAuthFlow,
    'project-lifecycle': runProjectLifecycle,
    'asset-hub-crud': runAssetHubCrud,
    'novel-promotion': runNovelPromotion,
    'user-config': runUserConfig,
    'admin-billing': runAdminBilling,
  }

  for (const phaseId of PHASE_SEQUENCE) {
    const phase = createPhase(phaseId)
    report.phases.push(phase)
    const startedAtMs = Date.now()

    try {
      await phaseHandlers[phaseId](phase, context)
    } catch (error) {
      phase.status = 'FAIL'
      phase.error = error instanceof Error ? error.message : String(error)
    } finally {
      finalizePhase(phase, startedAtMs)
    }
  }

  report.userId = context.userId
  report.projectId = context.projectId

  await runCleanup(report, context)

  const { jsonPath, mdPath, summary } = writeReports(outDir, report)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(
    `PHASE_PASS=${summary.phasePass} PHASE_FAIL=${summary.phaseFail} STEP_PASS=${summary.stepPass} STEP_FAIL=${summary.stepFail} CLEANUP_FAIL=${summary.cleanupFail}`,
  )

  if (summary.phaseFail > 0 || summary.stepFail > 0 || summary.cleanupFail > 0) {
    process.exitCode = 1
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
