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
const PHASE_SEQUENCE = ['auth-flow', 'project-lifecycle', 'asset-hub-advanced']

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

function ensureArrayField(context, key) {
  if (!Array.isArray(context[key])) {
    context[key] = []
  }
  return context[key]
}

function removeTrackedAppearance(context, characterId, appearanceIndex, mode) {
  const refs = ensureArrayField(context, 'assetAppearanceRefs')
  context.assetAppearanceRefs = refs.filter(
    (item) =>
      !(
        isObject(item) &&
        item.characterId === characterId &&
        item.appearanceIndex === appearanceIndex &&
        item.mode === mode
      ),
  )
}

function assertErrorContract(body, contextLabel) {
  assert(body.success === false, `${contextLabel}: success should be false for error responses`)
  assert(typeof body.code === 'string' && body.code.trim().length > 0, `${contextLabel}: code is missing`)
  assert(typeof body.message === 'string' && body.message.trim().length > 0, `${contextLabel}: message is missing`)
  assert(isObject(body.error), `${contextLabel}: error payload is missing`)
}

function createIdentity() {
  const ts = Date.now()
  const nonce = crypto.randomUUID().slice(0, 8)
  const username = `e2e-asset-hub-advanced-${ts}-${nonce}@test.local`
  return {
    username,
    password: 'Test123456',
    projectName: `E2E Asset Hub Advanced ${ts}`,
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
          description: 'created by e2e-asset-hub-advanced',
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
}

export async function runAssetHubAdvancedPhase(phase, context) {
  const projectId = ensureProjectId(context)

  const characterCreate = await runStep(
    phase,
    {
      id: 'create-asset-character-advanced',
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
          name: `E2E Advanced Character ${Date.now()}`,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create advanced asset character')
      const body = expectJsonObject(response, 'create advanced asset character')
      assert(isObject(body.character), 'create advanced asset character: character payload missing')
      assert(typeof body.character.id === 'string' && body.character.id.trim().length > 0, 'create advanced asset character: id missing')

      ensureArrayField(context, 'assetCharacterIds').push(body.character.id)

      return {
        httpStatus: response.status,
        note: `characterId=${body.character.id}, projectId=${projectId}`,
        value: {
          characterId: body.character.id,
        },
      }
    },
  )

  const locationCreate = await runStep(
    phase,
    {
      id: 'create-asset-location-advanced',
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
          name: `E2E Advanced Location ${Date.now()}`,
          summary: 'advanced location summary',
          description: 'advanced location description',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create advanced asset location')
      const body = expectJsonObject(response, 'create advanced asset location')
      assert(isObject(body.location), 'create advanced asset location: location payload missing')
      assert(typeof body.location.id === 'string' && body.location.id.trim().length > 0, 'create advanced asset location: id missing')

      ensureArrayField(context, 'assetLocationIds').push(body.location.id)

      return {
        httpStatus: response.status,
        note: `locationId=${body.location.id}`,
        value: {
          locationId: body.location.id,
        },
      }
    },
  )

  if (characterCreate.ok) {
    await runStep(
      phase,
      {
        id: 'get-asset-character-single',
        method: 'GET',
        path: `/api/asset-hub/characters/${characterCreate.value.characterId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterCreate.value.characterId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get advanced asset character single')
        const body = expectJsonObject(response, 'get advanced asset character single')
        assert(isObject(body.character), 'get advanced asset character single: character payload missing')
        assert(body.character.id === characterCreate.value.characterId, 'get advanced asset character single: id mismatch')

        return {
          httpStatus: response.status,
          note: `characterId=${body.character.id}`,
        }
      },
    )
  }

  if (locationCreate.ok) {
    await runStep(
      phase,
      {
        id: 'get-asset-location-single',
        method: 'GET',
        path: `/api/asset-hub/locations/${locationCreate.value.locationId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/asset-hub/locations/${encodeURIComponent(locationCreate.value.locationId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get advanced asset location single')
        const body = expectJsonObject(response, 'get advanced asset location single')
        assert(isObject(body.location), 'get advanced asset location single: location payload missing')
        assert(body.location.id === locationCreate.value.locationId, 'get advanced asset location single: id mismatch')

        return {
          httpStatus: response.status,
          note: `locationId=${body.location.id}`,
        }
      },
    )
  }

  await runStep(
    phase,
    {
      id: 'list-asset-folders-advanced',
      method: 'GET',
      path: '/api/asset-hub/folders',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/asset-hub/folders',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list advanced asset folders')
      const body = expectJsonObject(response, 'list advanced asset folders')
      assert(Array.isArray(body.folders), 'list advanced asset folders: folders must be an array')

      return {
        httpStatus: response.status,
        note: `folders=${body.folders.length}`,
      }
    },
  )

  const folderCreate = await runStep(
    phase,
    {
      id: 'create-asset-folder-advanced',
      method: 'POST',
      path: '/api/asset-hub/folders',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/asset-hub/folders',
        token: ensureToken(context),
        body: {
          name: `E2E Advanced Folder ${Date.now()}`,
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'create advanced asset folder')
      const body = expectJsonObject(response, 'create advanced asset folder')
      assert(body.success === true, 'create advanced asset folder: success must be true')
      assert(isObject(body.folder), 'create advanced asset folder: folder payload missing')
      assert(typeof body.folder.id === 'string' && body.folder.id.trim().length > 0, 'create advanced asset folder: id missing')

      ensureArrayField(context, 'assetFolderIds').push(body.folder.id)

      return {
        httpStatus: response.status,
        note: `folderId=${body.folder.id}`,
        value: {
          folderId: body.folder.id,
        },
      }
    },
  )

  if (folderCreate.ok) {
    await runStep(
      phase,
      {
        id: 'update-asset-folder-advanced',
        method: 'PATCH',
        path: `/api/asset-hub/folders/${folderCreate.value.folderId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: `/api/asset-hub/folders/${encodeURIComponent(folderCreate.value.folderId)}`,
          token: ensureToken(context),
          body: {
            name: 'E2E Advanced Folder Updated',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'update advanced asset folder')
        const body = expectJsonObject(response, 'update advanced asset folder')
        assert(body.success === true, 'update advanced asset folder: success must be true')
        assert(isObject(body.folder), 'update advanced asset folder: folder payload missing')
        assert(body.folder.id === folderCreate.value.folderId, 'update advanced asset folder: id mismatch')

        return {
          httpStatus: response.status,
          note: `folderId=${body.folder.id}`,
        }
      },
    )

    const folderDelete = await runStep(
      phase,
      {
        id: 'delete-asset-folder-advanced',
        method: 'DELETE',
        path: `/api/asset-hub/folders/${folderCreate.value.folderId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/folders/${encodeURIComponent(folderCreate.value.folderId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'delete advanced asset folder')
        const body = expectJsonObject(response, 'delete advanced asset folder')
        assert(body.success === true, 'delete advanced asset folder: success must be true')

        return {
          httpStatus: response.status,
          note: `folderId=${folderCreate.value.folderId}`,
        }
      },
    )

    if (folderDelete.ok) {
      context.assetFolderIds = ensureArrayField(context, 'assetFolderIds').filter(
        (id) => id !== folderCreate.value.folderId,
      )
    }
  }

  if (characterCreate.ok) {
    const characterId = characterCreate.value.characterId

    const scopedAppearanceCreate = await runStep(
      phase,
      {
        id: 'upsert-character-appearance-advanced',
        method: 'POST',
        path: `/api/asset-hub/characters/${characterId}/appearances/1`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}/appearances/1`,
          token: ensureToken(context),
          body: {
            description: 'advanced appearance created',
            changeReason: 'e2e-advanced-upsert',
            imageUrls: [],
            selectedIndex: 0,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'upsert advanced character appearance')
        const body = expectJsonObject(response, 'upsert advanced character appearance')
        assert(body.success === true, 'upsert advanced character appearance: success must be true')
        assert(isObject(body.appearance), 'upsert advanced character appearance: appearance payload missing')
        assert(body.appearance.characterId === characterId, 'upsert advanced character appearance: characterId mismatch')
        assert(body.appearance.appearanceIndex === 1, 'upsert advanced character appearance: appearanceIndex mismatch')

        ensureArrayField(context, 'assetAppearanceRefs').push({
          mode: 'scoped',
          characterId,
          appearanceIndex: 1,
        })

        return {
          httpStatus: response.status,
          note: `characterId=${characterId}, appearanceIndex=1`,
        }
      },
    )

    if (scopedAppearanceCreate.ok) {
      await runStep(
        phase,
        {
          id: 'patch-character-appearance-advanced',
          method: 'PATCH',
          path: `/api/asset-hub/characters/${characterId}/appearances/1`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'PATCH',
            endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}/appearances/1`,
            token: ensureToken(context),
            body: {
              description: 'advanced appearance updated',
              changeReason: 'e2e-advanced-patch',
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'patch advanced character appearance')
          const body = expectJsonObject(response, 'patch advanced character appearance')
          assert(body.success === true, 'patch advanced character appearance: success must be true')

          return {
            httpStatus: response.status,
            note: `characterId=${characterId}, appearanceIndex=1`,
          }
        },
      )

      const scopedAppearanceDelete = await runStep(
        phase,
        {
          id: 'delete-character-appearance-advanced',
          method: 'DELETE',
          path: `/api/asset-hub/characters/${characterId}/appearances/1`,
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'DELETE',
            endpointPath: `/api/asset-hub/characters/${encodeURIComponent(characterId)}/appearances/1`,
            token: ensureToken(context),
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'delete advanced character appearance')
          const body = expectJsonObject(response, 'delete advanced character appearance')
          assert(body.success === true, 'delete advanced character appearance: success must be true')

          return {
            httpStatus: response.status,
            note: `characterId=${characterId}, appearanceIndex=1`,
          }
        },
      )

      if (scopedAppearanceDelete.ok) {
        removeTrackedAppearance(context, characterId, 1, 'scoped')
      }
    }

    const standaloneAppearanceCreate = await runStep(
      phase,
      {
        id: 'create-appearance-standalone-advanced',
        method: 'POST',
        path: '/api/asset-hub/appearances',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: '/api/asset-hub/appearances',
          token: ensureToken(context),
          body: {
            characterId,
            appearanceIndex: 2,
            description: 'standalone appearance created',
            changeReason: 'e2e-standalone-create',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'create advanced standalone appearance')
        const body = expectJsonObject(response, 'create advanced standalone appearance')
        assert(body.success === true, 'create advanced standalone appearance: success must be true')
        assert(isObject(body.appearance), 'create advanced standalone appearance: appearance payload missing')
        assert(body.appearance.characterId === characterId, 'create advanced standalone appearance: characterId mismatch')
        assert(body.appearance.appearanceIndex === 2, 'create advanced standalone appearance: appearanceIndex mismatch')

        ensureArrayField(context, 'assetAppearanceRefs').push({
          mode: 'standalone',
          characterId,
          appearanceIndex: 2,
        })

        return {
          httpStatus: response.status,
          note: `characterId=${characterId}, appearanceIndex=2`,
        }
      },
    )

    if (standaloneAppearanceCreate.ok) {
      await runStep(
        phase,
        {
          id: 'patch-appearance-standalone-advanced',
          method: 'PATCH',
          path: '/api/asset-hub/appearances',
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'PATCH',
            endpointPath: '/api/asset-hub/appearances',
            token: ensureToken(context),
            body: {
              characterId,
              appearanceIndex: 2,
              description: 'standalone appearance updated',
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'patch advanced standalone appearance')
          const body = expectJsonObject(response, 'patch advanced standalone appearance')
          assert(body.success === true, 'patch advanced standalone appearance: success must be true')

          return {
            httpStatus: response.status,
            note: `characterId=${characterId}, appearanceIndex=2`,
          }
        },
      )

      const standaloneAppearanceDelete = await runStep(
        phase,
        {
          id: 'delete-appearance-standalone-advanced',
          method: 'DELETE',
          path: '/api/asset-hub/appearances',
        },
        async () => {
          const response = await requestJson({
            baseUrl: context.baseUrl,
            method: 'DELETE',
            endpointPath: '/api/asset-hub/appearances',
            token: ensureToken(context),
            body: {
              characterId,
              appearanceIndex: 2,
            },
            timeoutMs: context.timeoutMs,
          })

          expectStatus(response, 200, 'delete advanced standalone appearance')
          const body = expectJsonObject(response, 'delete advanced standalone appearance')
          assert(body.success === true, 'delete advanced standalone appearance: success must be true')

          return {
            httpStatus: response.status,
            note: `characterId=${characterId}, appearanceIndex=2`,
          }
        },
      )

      if (standaloneAppearanceDelete.ok) {
        removeTrackedAppearance(context, characterId, 2, 'standalone')
      }
    }

    await runStep(
      phase,
      {
        id: 'post-character-voice-advanced',
        method: 'POST',
        path: '/api/asset-hub/character-voice',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: '/api/asset-hub/character-voice',
          token: ensureToken(context),
          body: {
            characterId,
            voiceType: 'custom',
            customVoiceUrl: 'https://example.com/e2e-voice.wav',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'post advanced character voice')
        const body = expectJsonObject(response, 'post advanced character voice')
        assert(body.success === true, 'post advanced character voice: success must be true')

        return {
          httpStatus: response.status,
          note: `audioUrl=${body.audioUrl ?? 'null'}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'patch-character-voice-advanced',
        method: 'PATCH',
        path: '/api/asset-hub/character-voice',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'PATCH',
          endpointPath: '/api/asset-hub/character-voice',
          token: ensureToken(context),
          body: {
            characterId,
            voiceType: 'global',
            globalVoiceId: 'e2e-global-voice-id',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'patch advanced character voice')
        const body = expectJsonObject(response, 'patch advanced character voice')
        assert(body.success === true, 'patch advanced character voice: success must be true')

        return {
          httpStatus: response.status,
          note: 'voice updated',
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'picker-character-advanced',
        method: 'GET',
        path: '/api/asset-hub/picker?type=character',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: '/api/asset-hub/picker?type=character',
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'advanced picker character')
        const body = expectJsonObject(response, 'advanced picker character')
        assert(Array.isArray(body.characters), 'advanced picker character: characters must be an array')
        const found = body.characters.some((item) => isObject(item) && item.id === characterId)
        assert(found, `advanced picker character: character ${characterId} not found`)

        return {
          httpStatus: response.status,
          note: `characters=${body.characters.length}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'select-image-contract-advanced',
        method: 'POST',
        path: '/api/asset-hub/select-image',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: '/api/asset-hub/select-image',
          token: ensureToken(context),
          body: {
            type: 'character',
            id: characterId,
            appearanceIndex: 999,
            imageIndex: 0,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [400, 404], 'advanced select image contract')
        const body = expectJsonObject(response, 'advanced select image contract')
        assertErrorContract(body, 'advanced select image contract')

        return {
          httpStatus: response.status,
          note: `code=${body.code}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'undo-image-contract-advanced',
        method: 'POST',
        path: '/api/asset-hub/undo-image',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: '/api/asset-hub/undo-image',
          token: ensureToken(context),
          body: {
            type: 'character',
            id: characterId,
            appearanceIndex: 0,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [400, 404], 'advanced undo image contract')
        const body = expectJsonObject(response, 'advanced undo image contract')
        assertErrorContract(body, 'advanced undo image contract')

        return {
          httpStatus: response.status,
          note: `code=${body.code}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'update-asset-label-advanced',
        method: 'POST',
        path: '/api/asset-hub/update-asset-label',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: '/api/asset-hub/update-asset-label',
          token: ensureToken(context),
          body: {
            type: 'character',
            id: characterId,
            appearanceIndex: 0,
            newName: 'E2E Advanced Label',
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'advanced update asset label')
        const body = expectJsonObject(response, 'advanced update asset label')
        assert(body.success === true, 'advanced update asset label: success must be true')
        assert(Array.isArray(body.results), 'advanced update asset label: results must be an array')

        return {
          httpStatus: response.status,
          note: `results=${body.results.length}`,
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

  const appearanceRefs = [...ensureArrayField(context, 'assetAppearanceRefs')].reverse()
  for (const ref of appearanceRefs) {
    if (!isObject(ref)) {
      continue
    }
    const characterId = String(ref.characterId ?? '').trim()
    const appearanceIndex = Number(ref.appearanceIndex)
    if (!characterId || !Number.isInteger(appearanceIndex)) {
      continue
    }

    await runCleanupStep(
      cleanup,
      {
        id: `delete-appearance-${characterId}-${appearanceIndex}`,
        method: 'DELETE',
        path: '/api/asset-hub/appearances',
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: '/api/asset-hub/appearances',
          token: context.token,
          body: {
            characterId,
            appearanceIndex,
          },
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete appearance')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const folderIds = [...ensureArrayField(context, 'assetFolderIds')].reverse()
  for (const folderId of folderIds) {
    await runCleanupStep(
      cleanup,
      {
        id: `delete-asset-folder-${folderId}`,
        method: 'DELETE',
        path: `/api/asset-hub/folders/${folderId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'DELETE',
          endpointPath: `/api/asset-hub/folders/${encodeURIComponent(folderId)}`,
          token: context.token,
          timeoutMs: context.timeoutMs,
        })

        expectStatusOneOf(response, [200, 404], 'cleanup delete asset folder')

        return {
          httpStatus: response.status,
          note: 'done',
        }
      },
    )
  }

  const locationIds = [...ensureArrayField(context, 'assetLocationIds')].reverse()
  for (const locationId of locationIds) {
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

  const characterIds = [...ensureArrayField(context, 'assetCharacterIds')].reverse()
  for (const characterId of characterIds) {
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

  lines.push('# E2E Asset Hub Advanced Summary')
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
  const jsonPath = path.join(outDir, 'e2e-asset-hub-advanced-summary.json')
  const mdPath = path.join(outDir, 'e2e-asset-hub-advanced-summary.md')

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
    console.log(`[dry-run] report: ${path.join(outDir, 'e2e-asset-hub-advanced-summary.md')}`)
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
    assetCharacterIds: [],
    assetLocationIds: [],
    assetFolderIds: [],
    assetAppearanceRefs: [],
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
    'asset-hub-advanced': runAssetHubAdvancedPhase,
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

const isDirectExecution =
  process.argv[1] &&
  path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname)

if (isDirectExecution) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error))
    process.exitCode = 1
  })
}
