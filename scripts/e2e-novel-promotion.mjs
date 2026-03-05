#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 20000

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {
    keepProject: false,
    dryRun: false,
  }

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)

    if (token === '--keep-project') {
      args.keepProject = true
      continue
    }
    if (token === '--dry-run') {
      args.dryRun = true
      continue
    }

    const key = token.slice(2)
    const value = argv[index + 1]
    assert(value && !value.startsWith('--'), `Missing value for --${key}`)
    args[key] = value
    index += 1
  }

  return args
}

function normalizeBaseUrl(raw) {
  assert(typeof raw === 'string' && raw.trim().length > 0, 'base is required (--base)')
  const normalized = raw.trim()
  assert(/^https?:\/\//.test(normalized), 'base must start with http:// or https://')
  return normalized.endsWith('/') ? normalized : `${normalized}/`
}

function parsePositiveInt(raw, name) {
  if (raw === undefined) {
    return undefined
  }
  const parsed = Number(raw)
  assert(Number.isInteger(parsed) && parsed > 0, `${name} must be a positive integer`)
  return parsed
}

function normalizePath(rawPath) {
  if (rawPath.startsWith('/')) {
    return rawPath.slice(1)
  }
  return rawPath
}

function formatError(error) {
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

function ensureObject(value, name) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), `${name} must be an object`)
  return value
}

function ensureArray(value, name) {
  assert(Array.isArray(value), `${name} must be an array`)
  return value
}

function ensureString(value, name) {
  assert(typeof value === 'string' && value.trim().length > 0, `${name} must be a non-empty string`)
  return value
}

function ensureTrue(value, name) {
  assert(value === true, `${name} must be true`)
  return true
}

function parseJsonSafely(raw) {
  if (typeof raw !== 'string' || raw.trim().length === 0) {
    return null
  }
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function summarizeBody(rawBody) {
  if (typeof rawBody !== 'string' || rawBody.length === 0) {
    return ''
  }
  const singleLine = rawBody.replace(/\s+/g, ' ').trim()
  if (singleLine.length <= 240) {
    return singleLine
  }
  return `${singleLine.slice(0, 237)}...`
}

async function requestJson({
  baseUrl,
  token,
  method,
  route,
  body,
  expectedStatus,
  timeoutMs,
}) {
  const targetUrl = new URL(normalizePath(route), baseUrl).toString()
  const expected = Array.isArray(expectedStatus) ? expectedStatus : [expectedStatus]

  const headers = {
    Accept: 'application/json',
  }
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }
  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)

  let response
  try {
    response = await fetch(targetUrl, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
    })
  } catch (error) {
    clearTimeout(timer)
    throw new Error(`${method} ${route} request failed: ${formatError(error)}`)
  }

  clearTimeout(timer)

  const rawBody = await response.text()
  const parsed = parseJsonSafely(rawBody)

  if (!expected.includes(response.status)) {
    const detail = summarizeBody(rawBody)
    throw new Error(
      `${method} ${route} expected status ${expected.join('/')} but got ${response.status}${detail ? ` body=${detail}` : ''}`,
    )
  }

  return {
    method,
    route,
    status: response.status,
    json: parsed,
    rawBody,
    url: targetUrl,
  }
}

function compactDetail(detail) {
  if (!detail || typeof detail !== 'object') {
    return ''
  }

  const pairs = []
  for (const [key, value] of Object.entries(detail)) {
    let formatted
    if (Array.isArray(value)) {
      formatted = `[${value.map((item) => String(item)).join(',')}]`
    } else if (value !== null && typeof value === 'object') {
      formatted = JSON.stringify(value)
    } else {
      formatted = String(value)
    }
    pairs.push(`${key}=${formatted}`)
  }
  return pairs.join('; ')
}

function escapeCell(value) {
  return String(value).replace(/\|/g, '\\|')
}

function renderMarkdownReport({
  baseUrl,
  username,
  projectId,
  keepProject,
  startedAt,
  finishedAt,
  steps,
  cleanupRows,
  flowError,
}) {
  const executed = steps.length
  const pass = steps.filter((step) => step.status === 'PASS').length
  const fail = steps.filter((step) => step.status === 'FAIL').length
  const cleanupPass = cleanupRows.filter((row) => row.status === 'PASS').length
  const cleanupFail = cleanupRows.filter((row) => row.status === 'FAIL').length
  const cleanupSkip = cleanupRows.filter((row) => row.status === 'SKIP').length

  const lines = []
  lines.push('# Novel Promotion E2E Report')
  lines.push('')
  lines.push(`- Started at: ${startedAt}`)
  lines.push(`- Finished at: ${finishedAt}`)
  lines.push(`- Base URL: ${baseUrl}`)
  lines.push(`- Test user: ${username}`)
  lines.push(`- Project ID: ${projectId || 'n/a'}`)
  lines.push(`- Keep project: ${keepProject}`)
  lines.push(`- Steps: total=${executed} pass=${pass} fail=${fail}`)
  lines.push(`- Cleanup: pass=${cleanupPass} fail=${cleanupFail} skip=${cleanupSkip}`)
  lines.push('')
  lines.push('## Route Mapping')
  lines.push('')
  lines.push('- Character CRUD: `POST/PATCH/DELETE /api/novel-promotion/{projectId}/character` (single-resource route in Rust)')
  lines.push('- Location CRUD: `POST/PATCH/DELETE /api/novel-promotion/{projectId}/location` (single-resource route in Rust)')
  lines.push('- Character/location listing: `GET /api/novel-promotion/{projectId}/assets`')
  lines.push('- Panel reading: `GET /api/novel-promotion/{projectId}/storyboards?episodeId=...` (panels are nested in storyboards payload)')
  lines.push('- Episode settings read/update: `GET/PUT /api/novel-promotion/{projectId}/editor?episodeId=...`')
  lines.push('- Project config read/update: `GET/PATCH /api/novel-promotion/{projectId}`')
  lines.push('')
  lines.push('## Flow Steps')
  lines.push('')
  lines.push('| # | Step | Result | Elapsed(ms) | Detail |')
  lines.push('|---|---|---|---|---|')

  for (const step of steps) {
    lines.push(
      `| ${escapeCell(step.id)} | ${escapeCell(step.name)} | ${step.status} | ${step.elapsedMs} | ${escapeCell(compactDetail(step.detail))} |`,
    )
  }

  lines.push('')
  lines.push('## Cleanup')
  lines.push('')
  lines.push('| Action | Result | Detail |')
  lines.push('|---|---|---|')
  for (const row of cleanupRows) {
    lines.push(
      `| ${escapeCell(row.action)} | ${row.status} | ${escapeCell(compactDetail(row.detail))} |`,
    )
  }

  lines.push('')
  lines.push('## Failure')
  lines.push('')
  if (!flowError) {
    lines.push('- none')
  } else {
    lines.push(`- ${flowError}`)
  }
  lines.push('')

  return `${lines.join('\n')}\n`
}

function writeReports(outDir, payload) {
  fs.mkdirSync(outDir, { recursive: true })
  const jsonPath = path.join(outDir, 'e2e-novel-promotion.json')
  const mdPath = path.join(outDir, 'e2e-novel-promotion.md')

  fs.writeFileSync(jsonPath, JSON.stringify(payload, null, 2), 'utf8')
  fs.writeFileSync(mdPath, renderMarkdownReport(payload), 'utf8')

  return { jsonPath, mdPath }
}

function createStepRunner(steps) {
  return async function runStep(id, name, action) {
    const startedAt = Date.now()
    try {
      const detail = await action()
      steps.push({
        id,
        name,
        status: 'PASS',
        elapsedMs: Date.now() - startedAt,
        detail: detail || {},
      })
    } catch (error) {
      steps.push({
        id,
        name,
        status: 'FAIL',
        elapsedMs: Date.now() - startedAt,
        detail: {
          error: formatError(error),
        },
      })
      throw error
    }
  }
}

function findById(items, id, label) {
  const found = items.find((item) => item && typeof item === 'object' && item.id === id)
  assert(found, `${label} with id=${id} not found`)
  return found
}

async function cleanupAction(cleanupRows, action, fn) {
  try {
    const detail = await fn()
    const isSkipped = detail && typeof detail === 'object' && detail.skipped === true
    cleanupRows.push({ action, status: isSkipped ? 'SKIP' : 'PASS', detail })
  } catch (error) {
    cleanupRows.push({
      action,
      status: 'FAIL',
      detail: {
        error: formatError(error),
      },
    })
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)
  const timeoutMs = parsePositiveInt(args['timeout-ms'], 'timeout-ms') || DEFAULT_TIMEOUT_MS

  const uniqueSuffix = `${Date.now()}${Math.floor(Math.random() * 1000).toString().padStart(3, '0')}`
  const username = args.username || `e2e_np_${uniqueSuffix}`
  const password = args.password || `E2E_np_${uniqueSuffix}`
  const projectName = args['project-name'] || `E2E Novel ${uniqueSuffix}`

  if (args.dryRun) {
    console.log(`[dry-run] base=${baseUrl}`)
    console.log(`[dry-run] outDir=${outDir}`)
    console.log(`[dry-run] username=${username}`)
    console.log(`[dry-run] projectName=${projectName}`)
    console.log(`[dry-run] keepProject=${args.keepProject}`)
    return
  }

  const startedAt = new Date().toISOString()
  const steps = []
  const cleanupRows = []
  const runStep = createStepRunner(steps)

  const context = {
    token: null,
    userId: null,
    projectId: null,
    episodeId: null,
    characterId: null,
    appearanceId: null,
    locationId: null,
    locationSelectedIndex: 0,
    storyboardId: null,
    panelId: null,
    editorRecordId: null,
    configGlobalAssetText: null,
  }

  let flowError = null

  try {
    await runStep('P0.1', 'Register user', async () => {
      const response = await requestJson({
        baseUrl,
        token: null,
        method: 'POST',
        route: '/api/auth/register',
        body: {
          name: username,
          password,
        },
        expectedStatus: 201,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'register response')
      const token = ensureString(payload.token, 'register.token')
      const user = ensureObject(payload.user, 'register.user')
      context.token = token
      context.userId = ensureString(user.id, 'register.user.id')

      return {
        status: response.status,
        userId: context.userId,
      }
    })

    await runStep('P0.2', 'Create project', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: '/api/projects',
        body: {
          name: projectName,
          description: 'e2e novel promotion project',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'create project response')
      const project = ensureObject(payload.project, 'create project.project')
      context.projectId = ensureString(project.id, 'create project.project.id')
      const mode = ensureString(project.mode, 'create project.project.mode')
      assert(mode === 'novel-promotion', `expected project mode novel-promotion but got ${mode}`)

      return {
        projectId: context.projectId,
        mode,
      }
    })

    await runStep('01', 'GET project root', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'novel root response')
      const project = ensureObject(payload.project, 'novel root project')
      const projectId = ensureString(project.id, 'novel root project.id')
      assert(projectId === context.projectId, `project id mismatch ${projectId} != ${context.projectId}`)

      const novelPromotionData = ensureObject(project.novelPromotionData, 'novel root project.novelPromotionData')
      const episodes = ensureArray(novelPromotionData.episodes, 'novel root episodes')

      return {
        episodes: episodes.length,
      }
    })

    await runStep('02', 'GET project assets', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/assets`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'assets response')
      const characters = ensureArray(payload.characters, 'assets.characters')
      const locations = ensureArray(payload.locations, 'assets.locations')

      return {
        characters: characters.length,
        locations: locations.length,
      }
    })

    await runStep('03', 'POST episode', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/episodes`,
        body: {
          name: 'E2E Episode',
          novelText: '\u6d4b\u8bd5\u5c0f\u8bf4\u5185\u5bb9...',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'create episode response')
      const episode = ensureObject(payload.episode, 'create episode.episode')
      context.episodeId = ensureString(episode.id, 'create episode.episode.id')

      return {
        episodeId: context.episodeId,
        episodeNumber: episode.episodeNumber,
      }
    })

    await runStep('04', 'GET episodes and verify create', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/episodes`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'list episodes response')
      const episodes = ensureArray(payload.episodes, 'list episodes.episodes')
      const createdEpisode = findById(episodes, context.episodeId, 'episode')
      assert(createdEpisode.name === 'E2E Episode', 'created episode name mismatch')

      return {
        count: episodes.length,
        episodeId: context.episodeId,
      }
    })

    await runStep('05', 'PATCH episode name', async () => {
      const updatedName = 'E2E Episode Updated'
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'PATCH',
        route: `/api/novel-promotion/${context.projectId}/episodes/${context.episodeId}`,
        body: {
          name: updatedName,
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'patch episode response')
      const episode = ensureObject(payload.episode, 'patch episode.episode')
      assert(episode.name === updatedName, `episode name was not updated to ${updatedName}`)

      return {
        episodeId: context.episodeId,
        name: episode.name,
      }
    })

    await runStep('06', 'POST character', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/character`,
        body: {
          name: '\u4e3b\u89d2',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'create character response')
      ensureTrue(payload.success, 'create character.success')
      const character = ensureObject(payload.character, 'create character.character')
      context.characterId = ensureString(character.id, 'create character.character.id')

      return {
        characterId: context.characterId,
      }
    })

    await runStep('07', 'GET characters and verify create', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/assets`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'assets response')
      const characters = ensureArray(payload.characters, 'assets.characters')
      const character = findById(characters, context.characterId, 'character')
      const appearances = ensureArray(character.appearances, 'character.appearances')
      assert(appearances.length > 0, 'character should have at least one appearance')
      context.appearanceId = ensureString(appearances[0].id, 'character.appearances[0].id')

      return {
        characters: characters.length,
        appearanceId: context.appearanceId,
      }
    })

    await runStep('08', 'PATCH character', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'PATCH',
        route: `/api/novel-promotion/${context.projectId}/character`,
        body: {
          characterId: context.characterId,
          name: '\u4e3b\u89d2-\u66f4\u65b0',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'patch character response')
      ensureTrue(payload.success, 'patch character.success')

      return {
        characterId: context.characterId,
      }
    })

    await runStep('09', 'POST location', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/location`,
        body: {
          name: '\u57ce\u5821',
          summary: 'e2e location summary',
          // Use a full-width comma so normalize_location_description resolves to an empty string,
          // preventing worker-side model invocation for this CRUD-only E2E script.
          description: '\uFF0C',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'create location response')
      ensureTrue(payload.success, 'create location.success')
      const location = ensureObject(payload.location, 'create location.location')
      context.locationId = ensureString(location.id, 'create location.location.id')

      return {
        locationId: context.locationId,
      }
    })

    await runStep('10', 'GET locations and verify create', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/assets`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'assets response')
      const locations = ensureArray(payload.locations, 'assets.locations')
      const location = findById(locations, context.locationId, 'location')
      const images = ensureArray(location.images, 'location.images')
      assert(images.length > 0, 'location should include at least one image record')
      const selected = images.find((item) => item.isSelected === true) || images[0]
      assert(Number.isInteger(selected.imageIndex), 'location selected imageIndex must be integer')
      context.locationSelectedIndex = selected.imageIndex

      return {
        locations: locations.length,
        selectedIndex: context.locationSelectedIndex,
      }
    })

    await runStep('11', 'GET storyboards (allow empty)', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/storyboards?episodeId=${encodeURIComponent(context.episodeId)}`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'storyboards response')
      const storyboards = ensureArray(payload.storyboards, 'storyboards.storyboards')

      return {
        storyboards: storyboards.length,
      }
    })

    await runStep('12', 'Create storyboard group for panel update', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/storyboard-group`,
        body: {
          episodeId: context.episodeId,
          insertIndex: 0,
          summary: 'e2e storyboard group',
          content: 'e2e clip content',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'create storyboard-group response')
      ensureTrue(payload.success, 'create storyboard-group.success')
      const storyboard = ensureObject(payload.storyboard, 'create storyboard-group.storyboard')
      const panel = ensureObject(payload.panel, 'create storyboard-group.panel')
      context.storyboardId = ensureString(storyboard.id, 'create storyboard-group.storyboard.id')
      context.panelId = ensureString(panel.id, 'create storyboard-group.panel.id')

      return {
        storyboardId: context.storyboardId,
        panelId: context.panelId,
      }
    })

    await runStep('13', 'Read panels via storyboards payload', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/storyboards?episodeId=${encodeURIComponent(context.episodeId)}`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'storyboards response')
      const storyboards = ensureArray(payload.storyboards, 'storyboards.storyboards')
      const storyboard = findById(storyboards, context.storyboardId, 'storyboard')
      const panels = ensureArray(storyboard.panels, 'storyboard.panels')
      const panel = findById(panels, context.panelId, 'panel')

      return {
        storyboards: storyboards.length,
        panels: panels.length,
        panelNumber: panel.panelNumber,
      }
    })

    await runStep('14', 'Select character image', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/select-character-image`,
        body: {
          appearanceId: context.appearanceId,
          selectedIndex: 0,
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'select-character-image response')
      ensureTrue(payload.success, 'select-character-image.success')

      return {
        appearanceId: context.appearanceId,
        selectedIndex: payload.selectedIndex,
      }
    })

    await runStep('15', 'Select location image', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'POST',
        route: `/api/novel-promotion/${context.projectId}/select-location-image`,
        body: {
          locationId: context.locationId,
          selectedIndex: context.locationSelectedIndex,
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'select-location-image response')
      ensureTrue(payload.success, 'select-location-image.success')

      return {
        locationId: context.locationId,
        selectedIndex: payload.selectedIndex,
      }
    })

    await runStep('16', 'Episode settings read+update via editor', async () => {
      const getResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/editor?episodeId=${encodeURIComponent(context.episodeId)}`,
        expectedStatus: 200,
        timeoutMs,
      })
      const getPayload = ensureObject(getResponse.json, 'editor GET response')
      assert(getPayload.episodeId === context.episodeId, 'editor GET episodeId mismatch')

      const putResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'PUT',
        route: `/api/novel-promotion/${context.projectId}/editor`,
        body: {
          episodeId: context.episodeId,
          projectData: {
            e2e: true,
            schemaVersion: 1,
            note: 'editor settings update check',
          },
          renderStatus: 'draft',
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const putPayload = ensureObject(putResponse.json, 'editor PUT response')
      ensureTrue(putPayload.success, 'editor PUT success')
      context.editorRecordId = ensureString(putPayload.id, 'editor PUT id')

      return {
        episodeId: context.episodeId,
        editorId: context.editorRecordId,
      }
    })

    await runStep('17', 'Update panel via /panel', async () => {
      const updatedDescription = 'e2e updated panel description'
      const updateResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'PUT',
        route: `/api/novel-promotion/${context.projectId}/panel`,
        body: {
          panelId: context.panelId,
          description: updatedDescription,
          linkedToNextPanel: false,
        },
        expectedStatus: 200,
        timeoutMs,
      })
      const updatePayload = ensureObject(updateResponse.json, 'panel PUT response')
      ensureTrue(updatePayload.success, 'panel PUT success')

      const verifyResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}/storyboards?episodeId=${encodeURIComponent(context.episodeId)}`,
        expectedStatus: 200,
        timeoutMs,
      })
      const verifyPayload = ensureObject(verifyResponse.json, 'storyboards verify response')
      const storyboards = ensureArray(verifyPayload.storyboards, 'storyboards verify.storyboards')
      const storyboard = findById(storyboards, context.storyboardId, 'storyboard')
      const panels = ensureArray(storyboard.panels, 'storyboard verify.panels')
      const panel = findById(panels, context.panelId, 'panel')
      assert(panel.description === updatedDescription, 'panel description was not updated')

      return {
        panelId: context.panelId,
      }
    })

    await runStep('18', 'GET config via project root', async () => {
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}`,
        expectedStatus: 200,
        timeoutMs,
      })

      const payload = ensureObject(response.json, 'root config response')
      const project = ensureObject(payload.project, 'root config project')
      const novelPromotionData = ensureObject(project.novelPromotionData, 'root config novelPromotionData')
      ensureString(novelPromotionData.projectId, 'root config novelPromotionData.projectId')

      return {
        workflowMode: novelPromotionData.workflowMode,
        ttsRate: novelPromotionData.ttsRate,
      }
    })

    await runStep('19', 'PATCH config and verify', async () => {
      const updatedGlobalAssetText = `e2e-global-${Date.now()}`
      context.configGlobalAssetText = updatedGlobalAssetText

      const patchResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'PATCH',
        route: `/api/novel-promotion/${context.projectId}`,
        body: {
          globalAssetText: updatedGlobalAssetText,
        },
        expectedStatus: 200,
        timeoutMs,
      })

      const patchPayload = ensureObject(patchResponse.json, 'patch config response')
      ensureTrue(patchPayload.success, 'patch config.success')

      const verifyResponse = await requestJson({
        baseUrl,
        token: context.token,
        method: 'GET',
        route: `/api/novel-promotion/${context.projectId}`,
        expectedStatus: 200,
        timeoutMs,
      })
      const verifyPayload = ensureObject(verifyResponse.json, 'verify config response')
      const project = ensureObject(verifyPayload.project, 'verify config project')
      const novelPromotionData = ensureObject(project.novelPromotionData, 'verify config novelPromotionData')
      assert(
        novelPromotionData.globalAssetText === updatedGlobalAssetText,
        'globalAssetText did not match patched value',
      )

      return {
        globalAssetText: updatedGlobalAssetText,
      }
    })
  } catch (error) {
    flowError = formatError(error)
  } finally {
    await cleanupAction(cleanupRows, 'delete-storyboard-group', async () => {
      if (!context.storyboardId) {
        return { skipped: true, reason: 'storyboardId missing' }
      }
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'DELETE',
        route: `/api/novel-promotion/${context.projectId}/storyboard-group?storyboardId=${encodeURIComponent(context.storyboardId)}`,
        expectedStatus: [200, 404],
        timeoutMs,
      })
      return {
        status: response.status,
      }
    })

    await cleanupAction(cleanupRows, 'delete-episode', async () => {
      if (!context.episodeId) {
        return { skipped: true, reason: 'episodeId missing' }
      }
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'DELETE',
        route: `/api/novel-promotion/${context.projectId}/episodes/${context.episodeId}`,
        expectedStatus: [200, 404],
        timeoutMs,
      })
      return {
        status: response.status,
      }
    })

    await cleanupAction(cleanupRows, 'delete-character', async () => {
      if (!context.characterId) {
        return { skipped: true, reason: 'characterId missing' }
      }
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'DELETE',
        route: `/api/novel-promotion/${context.projectId}/character`,
        body: {
          characterId: context.characterId,
        },
        expectedStatus: [200, 404],
        timeoutMs,
      })
      return {
        status: response.status,
      }
    })

    await cleanupAction(cleanupRows, 'delete-location', async () => {
      if (!context.locationId) {
        return { skipped: true, reason: 'locationId missing' }
      }
      const response = await requestJson({
        baseUrl,
        token: context.token,
        method: 'DELETE',
        route: `/api/novel-promotion/${context.projectId}/location`,
        body: {
          locationId: context.locationId,
        },
        expectedStatus: [200, 404],
        timeoutMs,
      })
      return {
        status: response.status,
      }
    })

    if (args.keepProject) {
      cleanupRows.push({
        action: 'delete-project',
        status: 'SKIP',
        detail: {
          reason: '--keep-project set',
          projectId: context.projectId,
        },
      })
    } else {
      await cleanupAction(cleanupRows, 'delete-project', async () => {
        if (!context.projectId) {
          return { skipped: true, reason: 'projectId missing' }
        }
        const response = await requestJson({
          baseUrl,
          token: context.token,
          method: 'DELETE',
          route: `/api/projects/${context.projectId}`,
          expectedStatus: [200, 404],
          timeoutMs,
        })
        return {
          status: response.status,
        }
      })
    }

    const finishedAt = new Date().toISOString()
    const payload = {
      startedAt,
      finishedAt,
      baseUrl,
      username,
      projectId: context.projectId,
      keepProject: args.keepProject,
      timeoutMs,
      flowError,
      steps,
      cleanupRows,
    }
    const { jsonPath, mdPath } = writeReports(outDir, payload)

    console.log(`wrote ${jsonPath}`)
    console.log(`wrote ${mdPath}`)
    console.log(
      `steps_pass=${steps.filter((step) => step.status === 'PASS').length} steps_fail=${steps.filter((step) => step.status === 'FAIL').length} cleanup_fail=${cleanupRows.filter((row) => row.status === 'FAIL').length}`,
    )

    const hasStepFailure = steps.some((step) => step.status === 'FAIL')
    const hasCleanupFailure = cleanupRows.some((row) => row.status === 'FAIL')
    if (flowError || hasStepFailure || hasCleanupFailure) {
      process.exitCode = 2
    }
  }
}

main().catch((error) => {
  console.error(formatError(error))
  process.exitCode = 1
})
