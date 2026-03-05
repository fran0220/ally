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
const PHASE_SEQUENCE = ['auth-flow', 'project-lifecycle', 'novel-advanced']

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
  const username = `e2e-novel-advanced-${ts}-${nonce}@test.local`
  return {
    username,
    password: 'Test123456',
    projectName: `E2E Novel Advanced ${ts}`,
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
          description: 'created by e2e-novel-advanced',
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

function ensureStringArray(context, key) {
  if (!Array.isArray(context[key])) {
    context[key] = []
  }
  return context[key]
}

function trackUniqueId(list, id) {
  if (typeof id !== 'string' || id.trim().length === 0) {
    return
  }
  if (!list.includes(id)) {
    list.push(id)
  }
}

function ensureNovelAdvancedState(context) {
  if (!isObject(context.novelAdvancedState)) {
    context.novelAdvancedState = {
      primaryEpisodeId: null,
      secondaryEpisodeId: null,
      transientEpisodeId: null,
      characterId: null,
      locationId: null,
      voiceLineId: null,
      primaryStoryboardId: null,
      secondaryStoryboardId: null,
      primaryClipId: null,
      secondaryClipId: null,
      primaryPanelId: null,
      extraPanelId: null,
      globalCharacterId: null,
      globalLocationId: null,
    }
  }
  return context.novelAdvancedState
}

async function runNovelAdvancedPhase(phase, context) {
  const projectId = ensureProjectId(context)
  const state = ensureNovelAdvancedState(context)
  const episodeIds = ensureStringArray(context, 'novelAdvancedEpisodeIds')
  const characterIds = ensureStringArray(context, 'novelAdvancedCharacterIds')
  const locationIds = ensureStringArray(context, 'novelAdvancedLocationIds')
  const storyboardIds = ensureStringArray(context, 'novelAdvancedStoryboardIds')
  const globalCharacterIds = ensureStringArray(context, 'novelAdvancedGlobalCharacterIds')
  const globalLocationIds = ensureStringArray(context, 'novelAdvancedGlobalLocationIds')

  const createBatch = await runStep(
    phase,
    {
      id: 'create-episodes-batch',
      method: 'POST',
      path: `/api/novel-promotion/${projectId}/episodes/batch`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/batch`,
        token: ensureToken(context),
        body: {
          clearExisting: true,
          episodes: [
            {
              episodeNumber: 1,
              name: 'Advanced Episode One',
              novelText: 'Advanced batch content one.',
            },
            {
              episodeNumber: 2,
              name: 'Advanced Episode Two',
              novelText: 'Advanced batch content two.',
            },
          ],
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create episodes batch')
      const body = expectJsonObject(response, 'create episodes batch')
      assert(body.success === true, 'create episodes batch: success must be true')
      assert(Array.isArray(body.episodes), 'create episodes batch: episodes must be an array')
      assert(body.episodes.length >= 2, 'create episodes batch: expected at least 2 episodes')

      const first = body.episodes[0]
      const second = body.episodes[1]
      assert(isObject(first), 'create episodes batch: first episode missing')
      assert(isObject(second), 'create episodes batch: second episode missing')
      assert(typeof first.id === 'string' && first.id.trim().length > 0, 'create episodes batch: first id missing')
      assert(typeof second.id === 'string' && second.id.trim().length > 0, 'create episodes batch: second id missing')

      state.primaryEpisodeId = first.id
      state.secondaryEpisodeId = second.id
      trackUniqueId(episodeIds, first.id)
      trackUniqueId(episodeIds, second.id)

      return {
        httpStatus: response.status,
        note: `primary=${state.primaryEpisodeId}, secondary=${state.secondaryEpisodeId}`,
      }
    },
  )

  if (!createBatch.ok) {
    return
  }

  const splitByMarkers = await runStep(
    phase,
    {
      id: 'split-episodes-by-markers',
      method: 'POST',
      path: `/api/novel-promotion/${projectId}/episodes/split-by-markers`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/split-by-markers`,
        token: ensureToken(context),
        body: {
          content: 'Advanced split-only section.',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'split episodes by markers')
      const body = expectJsonObject(response, 'split episodes by markers')
      assert(body.success === true, 'split episodes by markers: success must be true')
      assert(Array.isArray(body.episodes), 'split episodes by markers: episodes must be an array')
      assert(body.episodes.length >= 1, 'split episodes by markers: expected at least 1 episode')
      const transient = body.episodes[0]
      assert(isObject(transient), 'split episodes by markers: transient episode missing')
      assert(typeof transient.id === 'string' && transient.id.trim().length > 0, 'split episodes by markers: transient id missing')
      state.transientEpisodeId = transient.id
      trackUniqueId(episodeIds, transient.id)

      return {
        httpStatus: response.status,
        note: `created=${body.episodes.length}`,
      }
    },
  )

  if (splitByMarkers.ok && state.primaryEpisodeId) {
    await runStep(
      phase,
      {
        id: 'get-episode',
        method: 'GET',
        path: `/api/novel-promotion/${projectId}/episodes/${state.primaryEpisodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/${encodeURIComponent(state.primaryEpisodeId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get episode')
        const body = expectJsonObject(response, 'get episode')
        assert(isObject(body.episode), 'get episode: episode payload missing')
        assert(body.episode.id === state.primaryEpisodeId, 'get episode: id mismatch')

        return {
          httpStatus: response.status,
          note: `episodeId=${body.episode.id}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'patch-episode',
        method: 'PATCH',
        path: `/api/novel-promotion/${projectId}/episodes/${state.primaryEpisodeId}`,
      },
      async () => {
        const updatedName = `Advanced Episode Updated ${Date.now()}`
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/${encodeURIComponent(state.primaryEpisodeId)}`,
          token: ensureToken(context),
          body: {
            name: updatedName,
            novelText: 'Advanced episode patched content.',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch episode')
        const body = expectJsonObject(response, 'patch episode')
        assert(isObject(body.episode), 'patch episode: episode payload missing')
        assert(body.episode.name === updatedName, 'patch episode: name did not update')

        return {
          httpStatus: response.status,
          note: `name=${body.episode.name}`,
        }
      },
    )
  }

  if (state.transientEpisodeId) {
    await runStep(
      phase,
      {
        id: 'delete-episode',
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/episodes/${state.transientEpisodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/${encodeURIComponent(state.transientEpisodeId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'delete episode')
        const body = expectJsonObject(response, 'delete episode')
        assert(body.success === true, 'delete episode: success must be true')

        return {
          httpStatus: response.status,
          note: `episodeId=${state.transientEpisodeId}`,
        }
      },
    )
  }

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
          name: `Advanced Character ${Date.now()}`,
          introduction: 'advanced character introduction',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create novel character')
      const body = expectJsonObject(response, 'create novel character')
      assert(body.success === true, 'create novel character: success must be true')
      assert(isObject(body.character), 'create novel character: character payload missing')
      assert(typeof body.character.id === 'string' && body.character.id.trim().length > 0, 'create novel character: id missing')

      state.characterId = body.character.id
      trackUniqueId(characterIds, state.characterId)

      return {
        httpStatus: response.status,
        note: `characterId=${state.characterId}`,
      }
    },
  )

  if (createCharacter.ok && state.characterId) {
    await runStep(
      phase,
      {
        id: 'patch-novel-character',
        method: 'PATCH',
        path: `/api/novel-promotion/${projectId}/characters`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/characters`,
          token: ensureToken(context),
          body: {
            characterId: state.characterId,
            name: 'Advanced Character Updated',
            introduction: 'advanced character introduction updated',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch novel character')
        const body = expectJsonObject(response, 'patch novel character')
        assert(body.success === true, 'patch novel character: success must be true')

        return {
          httpStatus: response.status,
          note: `characterId=${state.characterId}`,
        }
      },
    )
  }

  const createLocation = await runStep(
    phase,
    {
      id: 'create-novel-location',
      method: 'POST',
      path: `/api/novel-promotion/${projectId}/locations`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/locations`,
        token: ensureToken(context),
        body: {
          name: `Advanced Location ${Date.now()}`,
          summary: 'advanced location summary',
          description: 'advanced location description',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create novel location')
      const body = expectJsonObject(response, 'create novel location')
      assert(body.success === true, 'create novel location: success must be true')
      assert(isObject(body.location), 'create novel location: location payload missing')
      assert(typeof body.location.id === 'string' && body.location.id.trim().length > 0, 'create novel location: id missing')

      state.locationId = body.location.id
      trackUniqueId(locationIds, state.locationId)

      return {
        httpStatus: response.status,
        note: `locationId=${state.locationId}`,
      }
    },
  )

  if (createLocation.ok && state.locationId) {
    await runStep(
      phase,
      {
        id: 'patch-novel-location',
        method: 'PATCH',
        path: `/api/novel-promotion/${projectId}/locations`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/locations`,
          token: ensureToken(context),
          body: {
            locationId: state.locationId,
            name: 'Advanced Location Updated',
            summary: 'advanced location summary updated',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch novel location')
        const body = expectJsonObject(response, 'patch novel location')
        assert(body.success === true, 'patch novel location: success must be true')

        return {
          httpStatus: response.status,
          note: `locationId=${state.locationId}`,
        }
      },
    )
  }

  if (state.primaryEpisodeId) {
    await runStep(
      phase,
      {
        id: 'get-voice-lines',
        method: 'GET',
        path: `/api/novel-promotion/${projectId}/voice-lines?episodeId=${state.primaryEpisodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/voice-lines?episodeId=${encodeURIComponent(state.primaryEpisodeId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get voice lines')
        const body = expectJsonObject(response, 'get voice lines')
        assert(Array.isArray(body.voiceLines), 'get voice lines: voiceLines must be an array')

        return {
          httpStatus: response.status,
          note: `voiceLines=${body.voiceLines.length}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'create-voice-line',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/voice-lines`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/voice-lines`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            speaker: 'Narrator',
            content: 'This is an advanced dispatch voice line.',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'create voice line')
        const body = expectJsonObject(response, 'create voice line')
        assert(body.success === true, 'create voice line: success must be true')
        assert(isObject(body.voiceLine), 'create voice line: voiceLine payload missing')
        assert(typeof body.voiceLine.id === 'string' && body.voiceLine.id.trim().length > 0, 'create voice line: id missing')
        state.voiceLineId = body.voiceLine.id

        return {
          httpStatus: response.status,
          note: `voiceLineId=${state.voiceLineId}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'get-editor',
        method: 'GET',
        path: `/api/novel-promotion/${projectId}/editor?episodeId=${state.primaryEpisodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/editor?episodeId=${encodeURIComponent(state.primaryEpisodeId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get editor')
        const body = expectJsonObject(response, 'get editor')
        assert(body.episodeId === state.primaryEpisodeId, 'get editor: episodeId mismatch')

        return {
          httpStatus: response.status,
          note: `episodeId=${body.episodeId}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'put-editor',
        method: 'PUT',
        path: `/api/novel-promotion/${projectId}/editor`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PUT',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/editor`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            projectData: {
              schemaVersion: 1,
              note: 'advanced editor save',
              timeline: [],
            },
            renderStatus: 'draft',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'put editor')
        const body = expectJsonObject(response, 'put editor')
        assert(body.success === true, 'put editor: success must be true')
        assert(typeof body.id === 'string' && body.id.trim().length > 0, 'put editor: id missing')

        return {
          httpStatus: response.status,
          note: `editorId=${body.id}`,
        }
      },
    )
  }

  if (state.primaryEpisodeId) {
    const createPrimaryGroup = await runStep(
      phase,
      {
        id: 'create-storyboard-group-primary',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/storyboard-group`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboard-group`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            summary: 'advanced storyboard group A',
            content: 'advanced storyboard clip A',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'create storyboard group primary')
        const body = expectJsonObject(response, 'create storyboard group primary')
        assert(body.success === true, 'create storyboard group primary: success must be true')
        assert(isObject(body.storyboard), 'create storyboard group primary: storyboard missing')
        assert(isObject(body.clip), 'create storyboard group primary: clip missing')
        assert(isObject(body.panel), 'create storyboard group primary: panel missing')
        assert(typeof body.storyboard.id === 'string' && body.storyboard.id.trim().length > 0, 'create storyboard group primary: storyboard id missing')
        assert(typeof body.clip.id === 'string' && body.clip.id.trim().length > 0, 'create storyboard group primary: clip id missing')
        assert(typeof body.panel.id === 'string' && body.panel.id.trim().length > 0, 'create storyboard group primary: panel id missing')

        state.primaryStoryboardId = body.storyboard.id
        state.primaryClipId = body.clip.id
        state.primaryPanelId = body.panel.id
        trackUniqueId(storyboardIds, state.primaryStoryboardId)

        return {
          httpStatus: response.status,
          note: `storyboard=${state.primaryStoryboardId}, clip=${state.primaryClipId}`,
        }
      },
    )

    if (createPrimaryGroup.ok && state.primaryStoryboardId) {
      await runStep(
        phase,
        {
          id: 'patch-storyboards',
          method: 'PATCH',
          path: `/api/novel-promotion/${projectId}/storyboards`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'PATCH',
            endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboards`,
            token: ensureToken(context),
            body: {
              storyboardId: state.primaryStoryboardId,
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'patch storyboards')
          const body = expectJsonObject(response, 'patch storyboards')
          assert(body.success === true, 'patch storyboards: success must be true')

          return {
            httpStatus: response.status,
            note: `storyboardId=${state.primaryStoryboardId}`,
          }
        },
      )

      const createPanel = await runStep(
        phase,
        {
          id: 'create-panel',
          method: 'POST',
          path: `/api/novel-promotion/${projectId}/panel`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'POST',
            endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/panel`,
            token: ensureToken(context),
            body: {
              storyboardId: state.primaryStoryboardId,
              description: 'advanced panel created by e2e',
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'create panel')
          const body = expectJsonObject(response, 'create panel')
          assert(body.success === true, 'create panel: success must be true')
          assert(isObject(body.panel), 'create panel: panel payload missing')
          assert(typeof body.panel.id === 'string' && body.panel.id.trim().length > 0, 'create panel: panel id missing')
          state.extraPanelId = body.panel.id

          return {
            httpStatus: response.status,
            note: `panelId=${state.extraPanelId}`,
          }
        },
      )

      if (createPanel.ok && state.extraPanelId) {
        await runStep(
          phase,
          {
            id: 'patch-panel',
            method: 'PATCH',
            path: `/api/novel-promotion/${projectId}/panel`,
          },
          async () => {
            const response = await requestJson({
              baseUrl: context.baseUrl,
              method: 'PATCH',
              endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/panel`,
              token: ensureToken(context),
              body: {
                panelId: state.extraPanelId,
                description: 'advanced panel updated by e2e',
                linkedToNextPanel: false,
              },
              timeoutMs: context.timeoutMs,
            })

            expectStatus(response, 200, 'patch panel')
            const body = expectJsonObject(response, 'patch panel')
            assert(body.success === true, 'patch panel: success must be true')

            return {
              httpStatus: response.status,
              note: `panelId=${state.extraPanelId}`,
            }
          },
        )

        await runStep(
          phase,
          {
            id: 'delete-panel',
            method: 'DELETE',
            path: `/api/novel-promotion/${projectId}/panel?panelId=${state.extraPanelId}`,
          },
          async () => {
            const response = await requestJson({
              baseUrl: context.baseUrl,
              method: 'DELETE',
              endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/panel?panelId=${encodeURIComponent(state.extraPanelId)}`,
              token: ensureToken(context),
              timeoutMs: context.timeoutMs,
            })

            expectStatus(response, 200, 'delete panel')
            const body = expectJsonObject(response, 'delete panel')
            assert(body.success === true, 'delete panel: success must be true')

            return {
              httpStatus: response.status,
              note: `panelId=${state.extraPanelId}`,
            }
          },
        )
      }
    }

    const createSecondaryGroup = await runStep(
      phase,
      {
        id: 'create-storyboard-group-secondary',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/storyboard-group`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboard-group`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            summary: 'advanced storyboard group B',
            content: 'advanced storyboard clip B',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'create storyboard group secondary')
        const body = expectJsonObject(response, 'create storyboard group secondary')
        assert(body.success === true, 'create storyboard group secondary: success must be true')
        assert(isObject(body.storyboard), 'create storyboard group secondary: storyboard missing')
        assert(isObject(body.clip), 'create storyboard group secondary: clip missing')
        assert(typeof body.storyboard.id === 'string' && body.storyboard.id.trim().length > 0, 'create storyboard group secondary: storyboard id missing')
        assert(typeof body.clip.id === 'string' && body.clip.id.trim().length > 0, 'create storyboard group secondary: clip id missing')

        state.secondaryStoryboardId = body.storyboard.id
        state.secondaryClipId = body.clip.id
        trackUniqueId(storyboardIds, state.secondaryStoryboardId)

        return {
          httpStatus: response.status,
          note: `storyboard=${state.secondaryStoryboardId}, clip=${state.secondaryClipId}`,
        }
      },
    )

    if (createSecondaryGroup.ok && state.primaryClipId && state.secondaryClipId) {
      await runStep(
        phase,
        {
          id: 'put-storyboard-group',
          method: 'PUT',
          path: `/api/novel-promotion/${projectId}/storyboard-group`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'PUT',
            endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboard-group`,
            token: ensureToken(context),
            body: {
              currentClipId: state.secondaryClipId,
              targetClipId: state.primaryClipId,
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'put storyboard group')
          const body = expectJsonObject(response, 'put storyboard group')
          assert(body.success === true, 'put storyboard group: success must be true')

          return {
            httpStatus: response.status,
            note: `current=${state.secondaryClipId}, target=${state.primaryClipId}`,
          }
        },
      )
    }

    if (state.secondaryStoryboardId) {
      await runStep(
        phase,
        {
          id: 'delete-storyboard-group',
          method: 'DELETE',
          path: `/api/novel-promotion/${projectId}/storyboard-group?storyboardId=${state.secondaryStoryboardId}`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'DELETE',
            endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboard-group?storyboardId=${encodeURIComponent(state.secondaryStoryboardId)}`,
            token: ensureToken(context),
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'delete storyboard group')
          const body = expectJsonObject(response, 'delete storyboard group')
          assert(body.success === true, 'delete storyboard group: success must be true')

          return {
            httpStatus: response.status,
            note: `storyboardId=${state.secondaryStoryboardId}`,
          }
        },
      )
    }
  }

  if (state.primaryEpisodeId) {
    await runStep(
      phase,
      {
        id: 'post-clips',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/clips`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/clips`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            targetType: 'episode',
            targetId: state.primaryEpisodeId,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'post clips')
        const body = expectJsonObject(response, 'post clips')
        assert(body.success === true, 'post clips: success must be true')
        assert(typeof body.taskId === 'string' && body.taskId.trim().length > 0, 'post clips: taskId missing')

        return {
          httpStatus: response.status,
          note: `taskId=${body.taskId}`,
        }
      },
    )
  }

  if (state.primaryClipId) {
    await runStep(
      phase,
      {
        id: 'patch-clip',
        method: 'PATCH',
        path: `/api/novel-promotion/${projectId}/clips/${state.primaryClipId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/clips/${encodeURIComponent(state.primaryClipId)}`,
          token: ensureToken(context),
          body: {
            summary: 'advanced clip patched summary',
            content: 'advanced clip patched content',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch clip')
        const body = expectJsonObject(response, 'patch clip')
        assert(body.success === true, 'patch clip: success must be true')
        assert(isObject(body.clip), 'patch clip: clip payload missing')
        assert(body.clip.id === state.primaryClipId, 'patch clip: clip id mismatch')

        return {
          httpStatus: response.status,
          note: `clipId=${state.primaryClipId}`,
        }
      },
    )
  }

  if (state.primaryEpisodeId) {
    await runStep(
      phase,
      {
        id: 'get-speaker-voice',
        method: 'GET',
        path: `/api/novel-promotion/${projectId}/speaker-voice?episodeId=${state.primaryEpisodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/speaker-voice?episodeId=${encodeURIComponent(state.primaryEpisodeId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get speaker voice')
        const body = expectJsonObject(response, 'get speaker voice')
        assert(isObject(body.speakerVoices), 'get speaker voice: speakerVoices must be an object')

        return {
          httpStatus: response.status,
          note: `keys=${Object.keys(body.speakerVoices).length}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'patch-speaker-voice',
        method: 'PATCH',
        path: `/api/novel-promotion/${projectId}/speaker-voice`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/speaker-voice`,
          token: ensureToken(context),
          body: {
            episodeId: state.primaryEpisodeId,
            speakerVoices: {
              Narrator: {
                voiceType: 'uploaded',
                audioUrl: 'https://example.com/advanced-voice.mp3',
              },
            },
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch speaker voice')
        const body = expectJsonObject(response, 'patch speaker voice')
        assert(body.success === true, 'patch speaker voice: success must be true')

        return {
          httpStatus: response.status,
          note: `episodeId=${state.primaryEpisodeId}`,
        }
      },
    )
  }

  await runStep(
    phase,
    {
      id: 'patch-novel-root',
      method: 'PATCH',
      path: `/api/novel-promotion/${projectId}`,
    },
    async () => {
      const globalAssetText = `advanced-global-${Date.now()}`
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'PATCH',
        endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}`,
        token: ensureToken(context),
        body: {
          globalAssetText,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'patch novel root')
      const body = expectJsonObject(response, 'patch novel root')
      assert(body.success === true, 'patch novel root: success must be true')
      assert(isObject(body.project), 'patch novel root: project payload missing')

      return {
        httpStatus: response.status,
        note: `globalAssetText=${globalAssetText}`,
      }
    },
  )

  const createGlobalCharacter = await runStep(
    phase,
    {
      id: 'create-global-character',
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
          name: `Advanced Global Character ${Date.now()}`,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create global character')
      const body = expectJsonObject(response, 'create global character')
      assert(isObject(body.character), 'create global character: character payload missing')
      assert(typeof body.character.id === 'string' && body.character.id.trim().length > 0, 'create global character: id missing')
      state.globalCharacterId = body.character.id
      trackUniqueId(globalCharacterIds, state.globalCharacterId)

      return {
        httpStatus: response.status,
        note: `globalCharacterId=${state.globalCharacterId}`,
      }
    },
  )

  const createGlobalLocation = await runStep(
    phase,
    {
      id: 'create-global-location',
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
          name: `Advanced Global Location ${Date.now()}`,
          summary: 'advanced global location summary',
          description: 'advanced global location description',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create global location')
      const body = expectJsonObject(response, 'create global location')
      assert(isObject(body.location), 'create global location: location payload missing')
      assert(typeof body.location.id === 'string' && body.location.id.trim().length > 0, 'create global location: id missing')
      state.globalLocationId = body.location.id
      trackUniqueId(globalLocationIds, state.globalLocationId)

      return {
        httpStatus: response.status,
        note: `globalLocationId=${state.globalLocationId}`,
      }
    },
  )

  if (createGlobalCharacter.ok && state.characterId && state.globalCharacterId) {
    await runStep(
      phase,
      {
        id: 'copy-from-global-character',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/copy-from-global`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/copy-from-global`,
          token: ensureToken(context),
          body: {
            type: 'character',
            targetId: state.characterId,
            globalAssetId: state.globalCharacterId,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'copy from global character')
        const body = expectJsonObject(response, 'copy from global character')
        assert(body.success === true, 'copy from global character: success must be true')
        assert(isObject(body.character), 'copy from global character: character payload missing')
        assert(body.character.id === state.characterId, 'copy from global character: target id mismatch')

        return {
          httpStatus: response.status,
          note: `targetId=${state.characterId}`,
        }
      },
    )
  }

  if (createGlobalLocation.ok && state.locationId && state.globalLocationId) {
    await runStep(
      phase,
      {
        id: 'copy-from-global-location',
        method: 'POST',
        path: `/api/novel-promotion/${projectId}/copy-from-global`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/copy-from-global`,
          token: ensureToken(context),
          body: {
            type: 'location',
            targetId: state.locationId,
            globalAssetId: state.globalLocationId,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'copy from global location')
        const body = expectJsonObject(response, 'copy from global location')
        assert(body.success === true, 'copy from global location: success must be true')
        assert(isObject(body.location), 'copy from global location: location payload missing')
        assert(body.location.id === state.locationId, 'copy from global location: target id mismatch')

        return {
          httpStatus: response.status,
          note: `targetId=${state.locationId}`,
        }
      },
    )
  }

  if (state.characterId) {
    await runStep(
      phase,
      {
        id: 'delete-novel-character',
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/characters?characterId=${state.characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/characters?characterId=${encodeURIComponent(state.characterId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'delete novel character')
        const body = expectJsonObject(response, 'delete novel character')
        assert(body.success === true, 'delete novel character: success must be true')

        return {
          httpStatus: response.status,
          note: `characterId=${state.characterId}`,
        }
      },
    )
  }

  if (state.locationId) {
    await runStep(
      phase,
      {
        id: 'delete-novel-location',
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/locations?locationId=${state.locationId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/locations?locationId=${encodeURIComponent(state.locationId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'delete novel location')
        const body = expectJsonObject(response, 'delete novel location')
        assert(body.success === true, 'delete novel location: success must be true')

        return {
          httpStatus: response.status,
          note: `locationId=${state.locationId}`,
        }
      },
    )
  }
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

  const projectId = context.projectId
  const token = context.token
  if (!(token && projectId)) {
    cleanup.endedAt = new Date().toISOString()
    return
  }

  const characterIds = [...new Set(context.novelAdvancedCharacterIds || [])]
  for (const characterId of characterIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-novel-character-${characterId}`,
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/characters?characterId=${characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/characters?characterId=${encodeURIComponent(characterId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete novel character')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const locationIds = [...new Set(context.novelAdvancedLocationIds || [])]
  for (const locationId of locationIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-novel-location-${locationId}`,
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/locations?locationId=${locationId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/locations?locationId=${encodeURIComponent(locationId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete novel location')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const storyboardIds = [...new Set(context.novelAdvancedStoryboardIds || [])]
  for (const storyboardId of storyboardIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-storyboard-group-${storyboardId}`,
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/storyboard-group?storyboardId=${storyboardId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/storyboard-group?storyboardId=${encodeURIComponent(storyboardId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete storyboard group')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const episodeIds = [...new Set(context.novelAdvancedEpisodeIds || [])]
  for (const episodeId of episodeIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-episode-${episodeId}`,
        method: 'DELETE',
        path: `/api/novel-promotion/${projectId}/episodes/${episodeId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/novel-promotion/${encodeURIComponent(projectId)}/episodes/${encodeURIComponent(episodeId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete episode')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const globalCharacterIds = [...new Set(context.novelAdvancedGlobalCharacterIds || [])]
  for (const characterId of globalCharacterIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-global-character-${characterId}`,
        method: 'DELETE',
        path: `/api/asset-hub/characters/${characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete global character')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const globalLocationIds = [...new Set(context.novelAdvancedGlobalLocationIds || [])]
  for (const locationId of globalLocationIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-global-location-${locationId}`,
        method: 'DELETE',
        path: `/api/asset-hub/locations/${locationId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/locations/${encodeURIComponent(locationId)}`,
          token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete global location')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  await runCleanupStep(
    cleanup,
    {
      id: 'delete-project',
      method: 'DELETE',
      path: `/api/projects/${projectId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'DELETE',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}`,
        token,
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [200, 404], 'cleanup delete project')

      return {
        httpStatus: response.status,
        note: `projectId=${projectId}`,
      }
    },
  )

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

  lines.push('# E2E Novel Advanced Summary')
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
  const jsonPath = path.join(outDir, 'e2e-novel-advanced.json')
  const mdPath = path.join(outDir, 'e2e-novel-advanced.md')

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
    console.log(`[dry-run] report: ${path.join(outDir, 'e2e-novel-advanced.md')}`)
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
    novelAdvancedState: null,
    novelAdvancedEpisodeIds: [],
    novelAdvancedCharacterIds: [],
    novelAdvancedLocationIds: [],
    novelAdvancedStoryboardIds: [],
    novelAdvancedGlobalCharacterIds: [],
    novelAdvancedGlobalLocationIds: [],
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
    'novel-advanced': runNovelAdvancedPhase,
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

const IS_MAIN = process.argv[1]
  ? path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname)
  : false

if (IS_MAIN) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error))
    process.exitCode = 1
  })
}

export { runNovelAdvancedPhase }
