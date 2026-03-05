#!/usr/bin/env node

import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_BASE_URL = 'http://127.0.0.1:3001'

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {
    dryRun: false,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

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

function summarizeRows(rows) {
  return {
    total: rows.length,
    pass: rows.filter((row) => row.result === 'PASS').length,
    fail: rows.filter((row) => row.result === 'FAIL').length,
  }
}

function toReportPath(outDir) {
  return path.join(outDir, 'e2e-project-lifecycle.md')
}

function renderMarkdownReport({ generatedAt, baseUrl, username, projectId, rows }) {
  const summary = summarizeRows(rows)
  const lines = []

  lines.push('# E2E Project Lifecycle Report')
  lines.push('')
  lines.push(`- Generated at: ${generatedAt}`)
  lines.push(`- Base URL: ${baseUrl}`)
  lines.push(`- E2E user: ${username}`)
  lines.push(`- Project ID: ${projectId || 'N/A'}`)
  lines.push(`- Total: ${summary.total}`)
  lines.push(`- Pass: ${summary.pass}`)
  lines.push(`- Fail: ${summary.fail}`)
  lines.push('')
  lines.push('| Step | Result | Method | Path | HTTP Status | Duration(ms) | Notes |')
  lines.push('|---|---|---|---|---|---|---|')

  for (const row of rows) {
    lines.push(
      `| ${markdownCell(row.id)} | ${row.result} | ${row.method} | ${markdownCell(row.path)} | ${markdownCell(row.httpStatus || '')} | ${markdownCell(row.durationMs)} | ${markdownCell(row.note)} |`,
    )
  }

  lines.push('')
  lines.push('## Failures')
  lines.push('')

  const failed = rows.filter((row) => row.result === 'FAIL')
  if (failed.length === 0) {
    lines.push('- none')
  } else {
    for (const row of failed) {
      lines.push(`- ${row.id}: ${row.note}`)
    }
  }

  lines.push('')
  return `${lines.join('\n')}\n`
}

function writeReport(outDir, reportPayload) {
  fs.mkdirSync(outDir, { recursive: true })
  const mdPath = toReportPath(outDir)
  fs.writeFileSync(mdPath, renderMarkdownReport(reportPayload), 'utf8')
  return mdPath
}

async function requestJson({ baseUrl, method, endpointPath, token, body }) {
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

  const response = await fetch(targetUrl, {
    method,
    headers,
    body: payload,
  })

  const rawBody = await response.text()
  const parsed = safeJsonParse(rawBody)

  return {
    url: targetUrl,
    status: response.status,
    contentType: response.headers.get('content-type') || '',
    rawBody,
    json: parsed.ok ? parsed.value : null,
    parseError: parsed.ok ? null : 'response is not valid JSON',
  }
}

function expectJsonObject(response, context) {
  assert(response.parseError === null, `${context}: ${response.parseError}`)
  assert(isObject(response.json), `${context}: response body must be a JSON object`)
  return response.json
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

function extractProject(responseBody, context) {
  if (isObject(responseBody.project)) {
    return responseBody.project
  }
  if (typeof responseBody.id === 'string' && responseBody.id.trim().length > 0) {
    return responseBody
  }
  throw new Error(`${context}: response missing project object`)
}

async function runStep(rows, metadata, executor) {
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
    rows.push(row)
    return { ok: true, value: result.value ?? null }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    const row = {
      ...metadata,
      result: 'FAIL',
      httpStatus: '',
      durationMs: Date.now() - startedAt,
      note: message,
    }
    rows.push(row)
    return { ok: false, value: null }
  }
}

function buildIdentity() {
  const runTs = Date.now()
  const nonce = crypto.randomUUID().slice(0, 8)
  return {
    timestamp: runTs,
    username: `e2e-project-lifecycle-${runTs}-${nonce}@test.com`,
    password: 'Test123456',
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'] || DEFAULT_BASE_URL, 'base')
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)

  if (args.dryRun) {
    console.log(`[dry-run] base: ${baseUrl}`)
    console.log(`[dry-run] report: ${toReportPath(outDir)}`)
    console.log('[dry-run] steps: auth-register, create-invalid, create-project, list, search, get, get-data, get-assets, update, get-not-found, delete, verify-deleted')
    return
  }

  const identity = buildIdentity()
  const createdProjectName = `E2E Test Project ${identity.timestamp}`
  const updatedProjectName = 'Updated E2E'
  const missingProjectId = crypto.randomUUID()
  const rows = []

  let token = null
  let projectId = null

  const registerStep = await runStep(
    rows,
    {
      id: 'auth-register',
      method: 'POST',
      path: '/api/auth/register',
    },
    async () => {
      const response = await requestJson({
        baseUrl,
        method: 'POST',
        endpointPath: '/api/auth/register',
        body: {
          username: identity.username,
          name: identity.username,
          password: identity.password,
        },
      })

      expectStatusOneOf(response, [200, 201], 'register')
      const body = expectJsonObject(response, 'register')
      assert(typeof body.token === 'string' && body.token.trim().length > 0, 'register: token is missing')

      return {
        httpStatus: response.status,
        note: `registered ${identity.username}`,
        value: {
          token: body.token,
        },
      }
    },
  )

  if (registerStep.ok) {
    token = registerStep.value.token
  }

  await runStep(
    rows,
    {
      id: 'create-project-empty-name',
      method: 'POST',
      path: '/api/projects',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')

      const response = await requestJson({
        baseUrl,
        method: 'POST',
        endpointPath: '/api/projects',
        token,
        body: {
          name: '',
        },
      })

      expectStatus(response, 400, 'create project with empty name')
      expectJsonObject(response, 'create project with empty name')

      return {
        httpStatus: response.status,
        note: 'empty name rejected',
      }
    },
  )

  const createStep = await runStep(
    rows,
    {
      id: 'create-project',
      method: 'POST',
      path: '/api/projects',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')

      const response = await requestJson({
        baseUrl,
        method: 'POST',
        endpointPath: '/api/projects',
        token,
        body: {
          name: createdProjectName,
        },
      })

      expectStatus(response, 200, 'create project')
      const body = expectJsonObject(response, 'create project')
      const project = extractProject(body, 'create project')

      assert(typeof project.id === 'string' && project.id.trim().length > 0, 'create project: project.id is missing')
      assert(project.name === createdProjectName, `create project: expected name ${createdProjectName}, got ${project.name}`)

      return {
        httpStatus: response.status,
        note: `projectId=${project.id}`,
        value: {
          projectId: project.id,
        },
      }
    },
  )

  if (createStep.ok) {
    projectId = createStep.value.projectId
  }

  await runStep(
    rows,
    {
      id: 'list-projects',
      method: 'GET',
      path: '/api/projects?page=1&pageSize=20',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: '/api/projects?page=1&pageSize=20',
        token,
      })

      expectStatus(response, 200, 'list projects')
      const body = expectJsonObject(response, 'list projects')
      assert(Array.isArray(body.projects), 'list projects: projects must be an array')
      assert(isObject(body.pagination), 'list projects: pagination must be an object')

      const found = body.projects.some((project) => isObject(project) && project.id === projectId)
      assert(found, `list projects: project ${projectId} not found`)

      return {
        httpStatus: response.status,
        note: `listed=${body.projects.length}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'search-projects',
      method: 'GET',
      path: '/api/projects?page=1&pageSize=5&search=E2E',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: '/api/projects?page=1&pageSize=5&search=E2E',
        token,
      })

      expectStatus(response, 200, 'search projects')
      const body = expectJsonObject(response, 'search projects')
      assert(Array.isArray(body.projects), 'search projects: projects must be an array')

      const found = body.projects.some((project) => isObject(project) && project.id === projectId)
      assert(found, `search projects: project ${projectId} not found in search results`)

      return {
        httpStatus: response.status,
        note: `matches=${body.projects.length}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'get-project',
      method: 'GET',
      path: projectId ? `/api/projects/${projectId}` : '/api/projects/{id}',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}`,
        token,
      })

      expectStatus(response, 200, 'get project')
      const body = expectJsonObject(response, 'get project')
      const project = extractProject(body, 'get project')
      assert(project.id === projectId, `get project: expected id ${projectId}, got ${project.id}`)

      return {
        httpStatus: response.status,
        note: `name=${project.name}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'get-project-data',
      method: 'GET',
      path: projectId ? `/api/projects/${projectId}/data` : '/api/projects/{id}/data',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}/data`,
        token,
      })

      expectStatus(response, 200, 'get project data')
      const body = expectJsonObject(response, 'get project data')
      const project = extractProject(body, 'get project data')
      assert(project.id === projectId, `get project data: expected id ${projectId}, got ${project.id}`)
      assert(isObject(project.novelPromotionData), 'get project data: novelPromotionData must be an object')

      return {
        httpStatus: response.status,
        note: `novelPromotionData.id=${project.novelPromotionData.id}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'get-project-assets',
      method: 'GET',
      path: projectId ? `/api/projects/${projectId}/assets` : '/api/projects/{id}/assets',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}/assets`,
        token,
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

  await runStep(
    rows,
    {
      id: 'update-project',
      method: 'PATCH',
      path: projectId ? `/api/projects/${projectId}` : '/api/projects/{id}',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'PATCH',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}`,
        token,
        body: {
          name: updatedProjectName,
        },
      })

      expectStatus(response, 200, 'update project')
      const body = expectJsonObject(response, 'update project')
      const project = extractProject(body, 'update project')
      assert(project.id === projectId, `update project: expected id ${projectId}, got ${project.id}`)
      assert(project.name === updatedProjectName, `update project: expected name ${updatedProjectName}, got ${project.name}`)

      return {
        httpStatus: response.status,
        note: `name updated to ${updatedProjectName}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'get-project-not-found',
      method: 'GET',
      path: `/api/projects/${missingProjectId}`,
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(missingProjectId)}`,
        token,
      })

      expectStatus(response, 404, 'get non-existent project')
      expectJsonObject(response, 'get non-existent project')

      return {
        httpStatus: response.status,
        note: `missing project ${missingProjectId} returns 404`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'delete-project',
      method: 'DELETE',
      path: projectId ? `/api/projects/${projectId}` : '/api/projects/{id}',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'DELETE',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}`,
        token,
      })

      expectStatus(response, 200, 'delete project')
      const body = expectJsonObject(response, 'delete project')
      assert(body.success === true, 'delete project: success must be true')

      return {
        httpStatus: response.status,
        note: `cosFilesDeleted=${body.cosFilesDeleted ?? 'unknown'} cosFilesFailed=${body.cosFilesFailed ?? 'unknown'}`,
      }
    },
  )

  await runStep(
    rows,
    {
      id: 'verify-project-deleted',
      method: 'GET',
      path: projectId ? `/api/projects/${projectId}` : '/api/projects/{id}',
    },
    async () => {
      assert(typeof token === 'string' && token.trim().length > 0, 'missing auth token from register step')
      assert(typeof projectId === 'string' && projectId.trim().length > 0, 'missing project id from create step')

      const response = await requestJson({
        baseUrl,
        method: 'GET',
        endpointPath: `/api/projects/${encodeURIComponent(projectId)}`,
        token,
      })

      expectStatus(response, 404, 'verify project deleted')
      expectJsonObject(response, 'verify project deleted')

      return {
        httpStatus: response.status,
        note: `project ${projectId} no longer accessible`,
      }
    },
  )

  const reportPayload = {
    generatedAt: new Date().toISOString(),
    baseUrl,
    username: identity.username,
    projectId,
    rows,
  }

  const mdPath = writeReport(outDir, reportPayload)
  const summary = summarizeRows(rows)

  console.log(`wrote ${mdPath}`)
  console.log(`PASS=${summary.pass} FAIL=${summary.fail} TOTAL=${summary.total}`)

  if (summary.fail > 0) {
    process.exitCode = 1
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
