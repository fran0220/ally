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
const SUMMARY_JSON_FILE = 'e2e-tasks-runs-summary.json'
const SUMMARY_MD_FILE = 'e2e-tasks-runs-summary.md'
const SETUP_SEQUENCE = ['auth-flow', 'project-lifecycle']
const PHASE_SEQUENCE = ['tasks-api', 'runs-api']
const ALL_PHASE_SEQUENCE = [...SETUP_SEQUENCE, ...PHASE_SEQUENCE]

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {
    clean: true,
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
  const username = `e2e-tasks-runs-${ts}-${nonce}@test.local`
  return {
    username,
    password: 'Test123456',
    projectName: `E2E Tasks Runs ${ts}`,
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
          description: 'created by e2e-tasks-runs',
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
}

export async function runTasksPhase(phase, context) {
  const projectId = ensureProjectId(context)
  const targetType = 'episode'
  const targetId = context.novelEpisodeId || context.taskTargetId || `episode-${Date.now()}`
  const missingTaskId = crypto.randomUUID()
  context.taskTargetId = targetId

  const listTasksStep = await runStep(
    phase,
    {
      id: 'list-tasks-by-project',
      method: 'GET',
      path: `/api/tasks?projectId=${projectId}&limit=20`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/tasks?projectId=${encodeURIComponent(projectId)}&limit=20`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list tasks by project')
      const body = expectJsonObject(response, 'list tasks by project')
      assert(Array.isArray(body.tasks), 'list tasks by project: tasks must be an array')

      if (body.tasks.length > 0) {
        const first = body.tasks[0]
        assert(isObject(first), 'list tasks by project: task row must be an object')
        if (typeof first.id === 'string' && first.id.trim().length > 0) {
          context.taskId = first.id
        }
      }

      return {
        httpStatus: response.status,
        note: `tasks=${body.tasks.length}`,
      }
    },
  )

  if (!listTasksStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'list-tasks-by-filters',
      method: 'GET',
      path: `/api/tasks?projectId=${projectId}&targetType=${targetType}&targetId=${targetId}&status=queued&status=processing&type=story_to_script&limit=20`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/tasks?projectId=${encodeURIComponent(projectId)}&targetType=${encodeURIComponent(targetType)}&targetId=${encodeURIComponent(targetId)}&status=queued&status=processing&type=story_to_script&limit=20`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list tasks by filters')
      const body = expectJsonObject(response, 'list tasks by filters')
      assert(Array.isArray(body.tasks), 'list tasks by filters: tasks must be an array')

      return {
        httpStatus: response.status,
        note: `tasks=${body.tasks.length}`,
      }
    },
  )

  const targetStatesStep = await runStep(
    phase,
    {
      id: 'task-target-states',
      method: 'POST',
      path: '/api/task-target-states',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/task-target-states',
        token: ensureToken(context),
        body: {
          projectId,
          targets: [
            {
              targetType,
              targetId,
              types: ['story_to_script'],
            },
          ],
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'task target states')
      const body = expectJsonObject(response, 'task target states')
      assert(Array.isArray(body.states), 'task target states: states must be an array')
      assert(body.states.length === 1, 'task target states: expected one state row')

      const state = body.states[0]
      assert(isObject(state), 'task target states: state row must be an object')
      assert(state.targetType === targetType, 'task target states: targetType mismatch')
      assert(state.targetId === targetId, 'task target states: targetId mismatch')

      if (typeof state.taskId === 'string' && state.taskId.trim().length > 0) {
        context.taskId = state.taskId
      }

      return {
        httpStatus: response.status,
        note: `phase=${state.phase ?? 'unknown'} status=${state.status ?? 'unknown'}`,
      }
    },
  )

  if (!targetStatesStep.ok) {
    return
  }

  await runStep(
    phase,
    {
      id: 'dismiss-empty-task-ids',
      method: 'POST',
      path: '/api/tasks/dismiss',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/tasks/dismiss',
        token: ensureToken(context),
        body: {
          taskIds: [],
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 400, 'dismiss empty task ids')
      expectJsonObject(response, 'dismiss empty task ids')

      return {
        httpStatus: response.status,
        note: 'empty taskIds rejected as expected',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'dismiss-missing-task',
      method: 'POST',
      path: '/api/tasks/dismiss',
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/tasks/dismiss',
        token: ensureToken(context),
        body: {
          taskIds: [missingTaskId],
        },
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'dismiss missing task')
      const body = expectJsonObject(response, 'dismiss missing task')
      assert(body.success === true, 'dismiss missing task: success must be true')

      return {
        httpStatus: response.status,
        note: `dismissed=${body.dismissed ?? 'unknown'}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-missing-task-with-events',
      method: 'GET',
      path: `/api/tasks/${missingTaskId}?includeEvents=true&eventsLimit=10`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/tasks/${encodeURIComponent(missingTaskId)}?includeEvents=true&eventsLimit=10`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'get missing task')
      expectJsonObject(response, 'get missing task')

      return {
        httpStatus: response.status,
        note: 'missing task returns 404',
      }
    },
  )

  if (typeof context.taskId === 'string' && context.taskId.trim().length > 0) {
    await runStep(
      phase,
      {
        id: 'get-existing-task-with-events',
        method: 'GET',
        path: `/api/tasks/${context.taskId}?includeEvents=true&eventsLimit=10`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/tasks/${encodeURIComponent(context.taskId)}?includeEvents=true&eventsLimit=10`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get existing task')
        const body = expectJsonObject(response, 'get existing task')
        assert(isObject(body.task), 'get existing task: task payload missing')
        assert(body.task.id === context.taskId, 'get existing task: task id mismatch')

        return {
          httpStatus: response.status,
          note: `status=${body.task.status ?? 'unknown'}`,
        }
      },
    )
  }

  await runStep(
    phase,
    {
      id: 'delete-missing-task',
      method: 'DELETE',
      path: `/api/tasks/${missingTaskId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'DELETE',
        endpointPath: `/api/tasks/${encodeURIComponent(missingTaskId)}`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'delete missing task')
      expectJsonObject(response, 'delete missing task')

      return {
        httpStatus: response.status,
        note: 'missing task returns 404',
      }
    },
  )
}

export async function runRunsPhase(phase, context) {
  const projectId = ensureProjectId(context)
  const workflowType = 'story_to_script'
  const targetType = 'episode'
  const targetId = context.novelEpisodeId || context.taskTargetId || `episode-${Date.now()}`
  const missingRunId = crypto.randomUUID()

  const createRunStep = await runStep(
    phase,
    {
      id: 'create-run',
      method: 'POST',
      path: '/api/runs',
    },
    async () => {
      const payload = {
        projectId,
        workflowType,
        targetType,
        targetId,
        episodeId: targetId,
        taskType: 'story_to_script',
        input: {
          source: 'e2e-tasks-runs',
          createdAt: new Date().toISOString(),
        },
      }

      if (typeof context.taskId === 'string' && context.taskId.trim().length > 0) {
        payload.taskId = context.taskId
      }

      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: '/api/runs',
        token: ensureToken(context),
        body: payload,
        timeoutMs: context.timeoutMs,
      })

      expectStatusOneOf(response, [200, 201], 'create run')
      const body = expectJsonObject(response, 'create run')
      assert(body.success === true, 'create run: success must be true')
      assert(typeof body.runId === 'string' && body.runId.trim().length > 0, 'create run: runId missing')
      assert(isObject(body.run), 'create run: run payload missing')

      context.runId = body.runId
      context.runIds.push(body.runId)

      return {
        httpStatus: response.status,
        note: `runId=${body.runId} status=${body.run.status ?? 'unknown'}`,
        value: {
          runId: body.runId,
        },
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'list-runs-by-project',
      method: 'GET',
      path: `/api/runs?projectId=${projectId}&limit=20`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/runs?projectId=${encodeURIComponent(projectId)}&limit=20`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list runs by project')
      const body = expectJsonObject(response, 'list runs by project')
      assert(Array.isArray(body.runs), 'list runs by project: runs must be an array')

      if (createRunStep.ok) {
        const found = body.runs.some((item) => isObject(item) && item.id === createRunStep.value.runId)
        assert(found, `list runs by project: run ${createRunStep.value.runId} not found`)
      }

      return {
        httpStatus: response.status,
        note: `runs=${body.runs.length}`,
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'list-runs-by-filters',
      method: 'GET',
      path: `/api/runs?projectId=${projectId}&workflowType=${workflowType}&targetType=${targetType}&targetId=${targetId}&status=queued&status=canceling&limit=20`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/runs?projectId=${encodeURIComponent(projectId)}&workflowType=${encodeURIComponent(workflowType)}&targetType=${encodeURIComponent(targetType)}&targetId=${encodeURIComponent(targetId)}&status=queued&status=canceling&limit=20`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 200, 'list runs by filters')
      const body = expectJsonObject(response, 'list runs by filters')
      assert(Array.isArray(body.runs), 'list runs by filters: runs must be an array')

      return {
        httpStatus: response.status,
        note: `runs=${body.runs.length}`,
      }
    },
  )

  if (createRunStep.ok) {
    await runStep(
      phase,
      {
        id: 'get-run',
        method: 'GET',
        path: `/api/runs/${createRunStep.value.runId}`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/runs/${encodeURIComponent(createRunStep.value.runId)}`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get run')
        const body = expectJsonObject(response, 'get run')
        assert(isObject(body.run), 'get run: run payload missing')
        assert(body.run.id === createRunStep.value.runId, 'get run: run id mismatch')
        assert(Array.isArray(body.events), 'get run: events must be an array')

        return {
          httpStatus: response.status,
          note: `status=${body.run.status ?? 'unknown'} steps=${body.events.length}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'get-run-events',
        method: 'GET',
        path: `/api/runs/${createRunStep.value.runId}/events?afterSeq=0&limit=20`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/runs/${encodeURIComponent(createRunStep.value.runId)}/events?afterSeq=0&limit=20`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get run events')
        const body = expectJsonObject(response, 'get run events')
        assert(Array.isArray(body.events), 'get run events: events must be an array')

        return {
          httpStatus: response.status,
          note: `events=${body.events.length}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'cancel-run',
        method: 'POST',
        path: `/api/runs/${createRunStep.value.runId}/cancel`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'POST',
          endpointPath: `/api/runs/${encodeURIComponent(createRunStep.value.runId)}/cancel`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'cancel run')
        const body = expectJsonObject(response, 'cancel run')
        assert(body.success === true, 'cancel run: success must be true')
        assert(isObject(body.run), 'cancel run: run payload missing')
        assert(body.run.id === createRunStep.value.runId, 'cancel run: run id mismatch')

        return {
          httpStatus: response.status,
          note: `status=${body.run.status ?? 'unknown'}`,
        }
      },
    )

    await runStep(
      phase,
      {
        id: 'get-run-events-after-cancel',
        method: 'GET',
        path: `/api/runs/${createRunStep.value.runId}/events?afterSeq=0&limit=20`,
      },
      async () => {
        const response = await requestJson({
          baseUrl: context.baseUrl,
          method: 'GET',
          endpointPath: `/api/runs/${encodeURIComponent(createRunStep.value.runId)}/events?afterSeq=0&limit=20`,
          token: ensureToken(context),
          timeoutMs: context.timeoutMs,
        })

        expectStatus(response, 200, 'get run events after cancel')
        const body = expectJsonObject(response, 'get run events after cancel')
        assert(Array.isArray(body.events), 'get run events after cancel: events must be an array')

        return {
          httpStatus: response.status,
          note: `events=${body.events.length}`,
        }
      },
    )
  }

  await runStep(
    phase,
    {
      id: 'get-missing-run',
      method: 'GET',
      path: `/api/runs/${missingRunId}`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/runs/${encodeURIComponent(missingRunId)}`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'get missing run')
      expectJsonObject(response, 'get missing run')

      return {
        httpStatus: response.status,
        note: 'missing run returns 404',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'cancel-missing-run',
      method: 'POST',
      path: `/api/runs/${missingRunId}/cancel`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'POST',
        endpointPath: `/api/runs/${encodeURIComponent(missingRunId)}/cancel`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'cancel missing run')
      expectJsonObject(response, 'cancel missing run')

      return {
        httpStatus: response.status,
        note: 'missing run returns 404',
      }
    },
  )

  await runStep(
    phase,
    {
      id: 'get-missing-run-events',
      method: 'GET',
      path: `/api/runs/${missingRunId}/events?afterSeq=0&limit=20`,
    },
    async () => {
      const response = await requestJson({
        baseUrl: context.baseUrl,
        method: 'GET',
        endpointPath: `/api/runs/${encodeURIComponent(missingRunId)}/events?afterSeq=0&limit=20`,
        token: ensureToken(context),
        timeoutMs: context.timeoutMs,
      })

      expectStatus(response, 404, 'get missing run events')
      expectJsonObject(response, 'get missing run events')

      return {
        httpStatus: response.status,
        note: 'missing run returns 404',
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

  lines.push('# E2E Tasks Runs Summary')
  lines.push('')
  lines.push(`- Generated at: ${report.generatedAt}`)
  lines.push(`- Base URL: ${report.baseUrl}`)
  lines.push(`- Clean requested: ${report.cleanRequested}`)
  lines.push(`- Setup phases: ${SETUP_SEQUENCE.join(' -> ')}`)
  lines.push(`- Target phases: ${PHASE_SEQUENCE.join(' -> ')}`)
  lines.push(`- E2E user: ${report.identity.username}`)
  lines.push(`- User ID: ${report.userId || 'N/A'}`)
  lines.push(`- Project ID: ${report.projectId || 'N/A'}`)
  lines.push(`- Last run ID: ${report.runId || 'N/A'}`)
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
  const jsonPath = path.join(outDir, SUMMARY_JSON_FILE)
  const mdPath = path.join(outDir, SUMMARY_MD_FILE)

  const payload = {
    ...report,
    summary: buildSummary(report),
  }

  fs.writeFileSync(jsonPath, JSON.stringify(payload, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdown(payload), 'utf8')

  return { jsonPath, mdPath, summary: payload.summary }
}

function isDirectExecution() {
  if (!process.argv[1]) {
    return false
  }
  const currentPath = path.resolve(new URL(import.meta.url).pathname)
  const entryPath = path.resolve(process.argv[1])
  return currentPath === entryPath
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
    console.log(`[dry-run] report-json: ${path.join(outDir, SUMMARY_JSON_FILE)}`)
    console.log(`[dry-run] report-md: ${path.join(outDir, SUMMARY_MD_FILE)}`)
    console.log(`[dry-run] setup phases: ${SETUP_SEQUENCE.join(' -> ')}`)
    console.log(`[dry-run] test phases: ${PHASE_SEQUENCE.join(' -> ')}`)
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
    taskTargetId: null,
    taskId: null,
    runId: null,
    runIds: [],
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
    runId: null,
    phases: [],
    cleanup: null,
  }

  const phaseHandlers = {
    'auth-flow': runAuthFlow,
    'project-lifecycle': runProjectLifecycle,
    'tasks-api': runTasksPhase,
    'runs-api': runRunsPhase,
  }

  for (const phaseId of ALL_PHASE_SEQUENCE) {
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
  report.runId = context.runId

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

if (isDirectExecution()) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error))
    process.exitCode = 1
  })
}
