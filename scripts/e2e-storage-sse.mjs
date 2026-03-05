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
const DEFAULT_SSE_TIMEOUT_MS = 20000
const PHASE_SEQUENCE = ['auth-flow', 'project-lifecycle', 'storage-sse']
const REDIRECT_STATUSES = [301, 302, 303, 307, 308]

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
  const username = `e2e-storage-sse-${ts}-${nonce}@test.local`
  return {
    username,
    password: 'Test123456',
    projectName: `E2E Storage SSE ${ts}`,
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

async function requestRaw({
  baseUrl,
  method,
  endpointPath,
  token,
  body,
  headers,
  redirect,
  timeoutMs,
}) {
  const targetUrl = new URL(endpointPath, baseUrl).toString()
  const requestHeaders = {
    ...(headers || {}),
  }
  if (token) {
    requestHeaders.Authorization = `Bearer ${token}`
  }

  let payload
  if (body !== undefined) {
    requestHeaders['Content-Type'] = 'application/json'
    payload = JSON.stringify(body)
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  try {
    const response = await fetch(targetUrl, {
      method,
      headers: requestHeaders,
      body: payload,
      redirect,
      signal: controller.signal,
    })

    const rawBody = await response.text()
    const parsed = safeJsonParse(rawBody)
    const responseHeaders = {}
    for (const [key, value] of response.headers.entries()) {
      responseHeaders[key.toLowerCase()] = value
    }

    return {
      status: response.status,
      url: targetUrl,
      finalUrl: response.url,
      headers: responseHeaders,
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

function parseSseEventBlock(block) {
  const lines = block.split('\n')
  let eventName = 'message'
  let eventId = null
  const dataLines = []

  for (const rawLine of lines) {
    const line = rawLine.trimEnd()
    if (line.length === 0 || line.startsWith(':')) {
      continue
    }

    const sep = line.indexOf(':')
    const field = sep === -1 ? line : line.slice(0, sep)
    const value = sep === -1 ? '' : line.slice(sep + 1).trimStart()

    if (field === 'event') {
      eventName = value || 'message'
      continue
    }
    if (field === 'id') {
      eventId = value || null
      continue
    }
    if (field === 'data') {
      dataLines.push(value)
    }
  }

  const data = dataLines.join('\n')
  let json = null
  if (data.length > 0) {
    try {
      json = JSON.parse(data)
    } catch {
      json = null
    }
  }

  return {
    id: eventId,
    event: eventName,
    data,
    json,
  }
}

async function probeSse({ baseUrl, endpointPath, token, timeoutMs }) {
  const targetUrl = new URL(endpointPath, baseUrl).toString()
  const headers = {
    Accept: 'text/event-stream',
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  let reader = null

  try {
    const response = await fetch(targetUrl, {
      method: 'GET',
      headers,
      signal: controller.signal,
    })

    const contentType = response.headers.get('content-type') || ''
    if (response.status !== 200) {
      const rawBody = await response.text()
      const parsed = safeJsonParse(rawBody)
      return {
        status: response.status,
        url: targetUrl,
        contentType,
        rawBody,
        json: parsed.ok ? parsed.value : null,
        parseError: parsed.ok ? null : 'response is not valid JSON',
        heartbeatSeen: false,
        events: [],
      }
    }

    assert(contentType.includes('text/event-stream'), `unexpected SSE content-type: ${contentType}`)
    assert(response.body, 'sse response body is missing')

    reader = response.body.getReader()
    const decoder = new TextDecoder()
    let buffer = ''
    const events = []

    while (true) {
      const readResult = await reader.read()
      if (readResult.done) {
        break
      }

      const chunk = decoder.decode(readResult.value, { stream: true }).replace(/\r/g, '')
      buffer += chunk

      let marker = buffer.indexOf('\n\n')
      while (marker !== -1) {
        const block = buffer.slice(0, marker)
        buffer = buffer.slice(marker + 2)

        if (block.trim().length > 0) {
          const event = parseSseEventBlock(block)
          events.push(event.event)

          if (event.event === 'heartbeat') {
            return {
              status: response.status,
              url: targetUrl,
              contentType,
              rawBody: '',
              json: null,
              parseError: null,
              heartbeatSeen: true,
              events,
            }
          }
        }

        marker = buffer.indexOf('\n\n')
      }
    }

    return {
      status: response.status,
      url: targetUrl,
      contentType,
      rawBody: '',
      json: null,
      parseError: null,
      heartbeatSeen: false,
      events,
    }
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`request failed GET ${endpointPath}: timed out after ${timeoutMs}ms`)
    }
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`request failed GET ${endpointPath}: ${message}`)
  } finally {
    clearTimeout(timer)
    controller.abort()
    if (reader) {
      try {
        await reader.cancel()
      } catch {
        // ignore close errors
      }
    }
  }
}

function resolveSseTimeoutMs(context) {
  const raw = Number.isInteger(context.sseTimeoutMs) && context.sseTimeoutMs > 0
    ? context.sseTimeoutMs
    : context.timeoutMs
  return Math.min(20000, Math.max(5000, raw))
}

function expectCosRedirectOrMissingConfig(response, context) {
  if (REDIRECT_STATUSES.includes(response.status)) {
    const location = response.headers.location || ''
    assert(location.length > 0, `${context}: redirect response missing location header`)
    return {
      note: `redirect=${response.status} location=${location}`,
    }
  }

  expectStatus(response, 501, context)
  const body = expectJsonObject(response, context)
  assert(body.code === 'MISSING_CONFIG', `${context}: expected code=MISSING_CONFIG`)

  return {
    note: `missing-config code=${body.code}`,
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
          description: 'created by e2e-storage-sse',
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

export async function runStorageSsePhase(phase, context) {
  const projectId = ensureProjectId(context)
  const sseTimeoutMs = resolveSseTimeoutMs(context)

  await runStep(
    phase,
    {
      id: 'cos-image-auth',
      method: 'GET',
      path: '/api/cos/image?key=test.jpg',
    },
    async () => {
      const response = await requestRaw({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/cos/image?key=test.jpg',
        token: ensureToken(context),
        headers: {
          Accept: 'application/json',
        },
        redirect: 'manual',
        timeoutMs: context.timeoutMs,
      })

      const expectation = expectCosRedirectOrMissingConfig(response, 'cos image auth')

      return {
        httpStatus: response.status,
        note: expectation.note,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'cos-sign-auth',
      method: 'GET',
      path: '/api/cos/sign?key=test.jpg',
    },
    async () => {
      const response = await requestRaw({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/cos/sign?key=test.jpg',
        token: ensureToken(context),
        headers: {
          Accept: 'application/json',
        },
        redirect: 'manual',
        timeoutMs: context.timeoutMs,
      })

      const expectation = expectCosRedirectOrMissingConfig(response, 'cos sign auth')

      return {
        httpStatus: response.status,
        note: expectation.note,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'cos-image-no-auth',
      method: 'GET',
      path: '/api/cos/image?key=test.jpg',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/cos/image?key=test.jpg',
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [401, 404], 'cos image no auth')

      return {
        httpStatus: response.status,
        note: 'no auth rejected as expected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'cos-image-missing-key',
      method: 'GET',
      path: '/api/cos/image',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/cos/image',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 400, 'cos image missing key')
      expectJsonObject(response, 'cos image missing key')

      return {
        httpStatus: response.status,
        note: 'key param required',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'files-nonexistent',
      method: 'GET',
      path: '/api/files/nonexistent.png',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/files/nonexistent.png',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'files nonexistent')
      expectJsonObject(response, 'files nonexistent')

      return {
        httpStatus: response.status,
        note: 'not found as expected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'files-path-traversal-blocked',
      method: 'GET',
      path: '/api/files/../../etc/passwd',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/files/%2E%2E/%2E%2E/etc/passwd',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 403, 'files path traversal blocked')
      expectJsonObject(response, 'files path traversal blocked')

      return {
        httpStatus: response.status,
        note: 'path traversal rejected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'files-empty-path',
      method: 'GET',
      path: '/api/files/',
    },
    async () => {
      const response = await requestRaw({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/api/files/',
        token: ensureToken(context),
        headers: {
          Accept: 'application/json',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [400, 404, 500], 'files empty path')

      return {
        httpStatus: response.status,
        note: 'edge path handled',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'media-nonexistent-public-id',
      method: 'GET',
      path: '/m/nonexistent-public-id',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/m/nonexistent-public-id',
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'media nonexistent public id')
      expectJsonObject(response, 'media nonexistent public id')

      return {
        httpStatus: response.status,
        note: 'media not found as expected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'media-root-edge',
      method: 'GET',
      path: '/m/',
    },
    async () => {
      const response = await requestRaw({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: '/m/',
        token: ensureToken(context),
        headers: {
          Accept: 'application/json',
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [404, 405], 'media root edge')

      return {
        httpStatus: response.status,
        note: 'route edge handled',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'sse-connect-project-auth',
      method: 'GET',
      path: `/api/sse?projectId=${projectId}`,
    },
    async () => {
      const response = await probeSse({
        baseUrl: context.baseUrl,
        endpointPath: `/api/sse?projectId=${encodeURIComponent(projectId)}`,
        token: ensureToken(context),
        timeoutMs: sseTimeoutMs,
      })

      expectStatus(response, 200, 'sse connect project auth')
      assert(
        response.contentType.includes('text/event-stream'),
        `sse connect project auth: unexpected content-type ${response.contentType}`,
      )
      assert(
        response.heartbeatSeen,
        `sse connect project auth: expected heartbeat event within ${sseTimeoutMs}ms, events=${response.events.join(',') || 'none'}`,
      )

      return {
        httpStatus: response.status,
        note: `heartbeat observed events=${response.events.join(',') || 'none'}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'sse-without-project-id',
      method: 'GET',
      path: '/api/sse',
    },
    async () => {
      const response = await probeSse({
        baseUrl: context.baseUrl,
        endpointPath: '/api/sse',
        token: ensureToken(context),
        timeoutMs: sseTimeoutMs,
      })

      expectStatusOneOf(response, [200, 400], 'sse without project id')

      if (response.status === 200) {
        assert(
          response.contentType.includes('text/event-stream'),
          `sse without project id: unexpected content-type ${response.contentType}`,
        )
        assert(
          response.heartbeatSeen,
          `sse without project id: expected heartbeat event within ${sseTimeoutMs}ms, events=${response.events.join(',') || 'none'}`,
        )

        return {
          httpStatus: response.status,
          note: `connected events=${response.events.join(',') || 'none'}`,
        }
      }

      const body = expectJsonObject(response, 'sse without project id')
      assert(body.code || body.error, 'sse without project id: error payload missing code')

      return {
        httpStatus: response.status,
        note: `rejected code=${body.code || body.error}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'sse-without-auth',
      method: 'GET',
      path: `/api/sse?projectId=${projectId}`,
    },
    async () => {
      const response = await probeSse({
        baseUrl: context.baseUrl,
        endpointPath: `/api/sse?projectId=${encodeURIComponent(projectId)}`,
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 401, 'sse without auth')
      expectJsonObject(response, 'sse without auth')

      return {
        httpStatus: response.status,
        note: 'unauthorized as expected',
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

  lines.push('# E2E Storage SSE Summary')
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
  const jsonPath = path.join(outDir, 'e2e-storage-sse-summary.json')
  const mdPath = path.join(outDir, 'e2e-storage-sse-summary.md')

  const payload = {
    ...report,
    summary: buildSummary(report),
  }

  fs.writeFileSync(jsonPath, JSON.stringify(payload, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdown(payload), 'utf8')

  return { jsonPath, mdPath, summary: payload.summary }
}

function isMainModule() {
  if (!process.argv[1]) {
    return false
  }

  const cliEntry = path.resolve(process.argv[1])
  const modulePath = path.resolve(decodeURIComponent(new URL(import.meta.url).pathname))
  return cliEntry === modulePath
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'] || DEFAULT_BASE_URL, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms', DEFAULT_TIMEOUT_MS)
  const sseTimeoutMsRaw = parseOptionalInt(
    args['sse-timeout-ms'],
    'sse-timeout-ms',
    DEFAULT_SSE_TIMEOUT_MS,
  )
  const sseTimeoutMs = Math.min(20000, Math.max(5000, sseTimeoutMsRaw))

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] timeout-ms: ${timeoutMs}`)
    console.log(`[dry-run] sse-timeout-ms: ${sseTimeoutMs}`)
    console.log(`[dry-run] clean: ${args.clean}`)
    console.log(`[dry-run] report: ${path.join(outDir, 'e2e-storage-sse-summary.md')}`)
    console.log(`[dry-run] phases: ${PHASE_SEQUENCE.join(' -> ')}`)
    return
  }

  const identity = createIdentity()
  const context = {
    baseUrl,
    timeoutMs,
    sseTimeoutMs,
    clean: args.clean,
    identity,
    token: null,
    userId: null,
    projectId: null,
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
    'storage-sse': runStorageSsePhase,
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

if (isMainModule()) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error))
    process.exitCode = 1
  })
}
