#!/usr/bin/env node

import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname)
const PROJECT_ROOT = path.resolve(SCRIPT_DIR, '..')
const DEFAULT_OUT_DIR = path.join(PROJECT_ROOT, 'reports')
const DEFAULT_TIMEOUT_MS = 30000

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function parseArgs(argv) {
  const args = {}
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    assert(token.startsWith('--'), `Unsupported argument: ${token}`)
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

async function createSseConnection({ baseUrl, token, projectId, lastEventId }) {
  const controller = new AbortController()
  const decoder = new TextDecoder()

  const url = new URL('/api/sse', baseUrl)
  url.searchParams.set('projectId', projectId)

  const headers = {
    Accept: 'text/event-stream',
    Authorization: `Bearer ${token}`,
  }
  if (lastEventId) {
    headers['Last-Event-ID'] = String(lastEventId)
  }

  const response = await fetch(url.toString(), {
    method: 'GET',
    headers,
    signal: controller.signal,
  })

  const contentType = response.headers.get('content-type') || ''
  assert(response.status === 200, `SSE connect failed: status=${response.status}`)
  assert(contentType.includes('text/event-stream'), `unexpected SSE content-type: ${contentType}`)
  assert(response.body, 'SSE response body is missing')

  const reader = response.body.getReader()
  let buffer = ''

  function shiftEventFromBuffer() {
    const marker = buffer.indexOf('\n\n')
    if (marker === -1) return null
    const block = buffer.slice(0, marker)
    buffer = buffer.slice(marker + 2)
    if (block.trim().length === 0) {
      return null
    }
    return parseSseEventBlock(block)
  }

  async function nextEvent(timeoutMs) {
    const deadline = Date.now() + timeoutMs

    while (Date.now() < deadline) {
      const buffered = shiftEventFromBuffer()
      if (buffered) {
        return buffered
      }

      const readResult = await reader.read()
      if (readResult.done) {
        throw new Error('SSE stream closed before expected event')
      }

      const chunk = decoder.decode(readResult.value, { stream: true }).replace(/\r/g, '')
      buffer += chunk
    }

    throw new Error(`timeout waiting SSE event (${timeoutMs}ms)`)
  }

  async function close() {
    controller.abort()
    try {
      await reader.cancel()
    } catch {
      // ignore close errors during shutdown
    }
  }

  return {
    url: url.toString(),
    nextEvent,
    close,
  }
}

async function submitAssetHubTask({ baseUrl, token, suffix }) {
  const targetId = `sse-smoke-${suffix}-${Date.now()}`
  const url = new URL('/api/asset-hub/generate-image', baseUrl).toString()

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Accept-Language': 'en-US',
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({
      type: 'character',
      id: targetId,
      targetType: 'character',
      targetId,
      modelId: 'gpt-image-1',
      prompt: 'SSE reconnect smoke test image generation',
      meta: {
        locale: 'en',
      },
    }),
  })

  const raw = await response.text()
  let body = null
  try {
    body = JSON.parse(raw)
  } catch {
    body = null
  }

  assert(response.status === 200, `task submit failed status=${response.status} body=${raw}`)
  assert(body && typeof body.taskId === 'string' && body.taskId.trim().length > 0, 'task submit response missing taskId')

  return {
    url,
    taskId: body.taskId,
    body,
  }
}

async function waitForTaskLifecycleEvent({ connection, taskId, timeoutMs }) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const remaining = Math.max(1, deadline - Date.now())
    const event = await connection.nextEvent(remaining)
    if (event.event !== 'task.lifecycle') {
      continue
    }
    if (!event.json || typeof event.json !== 'object') {
      continue
    }
    if (event.json.taskId !== taskId) {
      continue
    }
    return event
  }
  throw new Error(`timeout waiting lifecycle event for task ${taskId}`)
}

function writeReports(outDir, report) {
  fs.mkdirSync(outDir, { recursive: true })
  const jsonPath = path.join(outDir, 'sse-reconnect-smoke.json')
  const mdPath = path.join(outDir, 'sse-reconnect-smoke.md')

  fs.writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf8')

  const lines = []
  lines.push('# SSE Reconnect Smoke Report')
  lines.push('')
  lines.push(`- Generated at: ${report.generatedAt}`)
  lines.push(`- Result: ${report.success ? 'PASS' : 'FAIL'}`)
  lines.push(`- Project ID: ${report.projectId}`)
  lines.push(`- First Task ID: ${report.firstTask?.taskId || 'N/A'}`)
  lines.push(`- Second Task ID: ${report.secondTask?.taskId || 'N/A'}`)
  lines.push(`- Last Event ID Before Disconnect: ${report.firstConnection?.lastEventId || 'N/A'}`)
  lines.push(`- Reconnect Event ID: ${report.secondConnection?.replayedEventId || 'N/A'}`)
  lines.push('')
  lines.push('## Assertions')
  lines.push('')
  for (const assertion of report.assertions) {
    lines.push(`- ${assertion}`)
  }
  if (!report.success && report.error) {
    lines.push('')
    lines.push('## Error')
    lines.push('')
    lines.push(`- ${report.error}`)
  }
  lines.push('')

  fs.writeFileSync(mdPath, `${lines.join('\n')}\n`, 'utf8')

  return { jsonPath, mdPath }
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const baseUrl = normalizeBaseUrl(args.base || args['rust-base'], 'base')
  const token = args.token || process.env.WW_TOKEN
  const projectId = args['project-id'] || process.env.WW_SSE_PROJECT_ID || 'global-asset-hub'
  const timeoutMs = parseOptionalInt(args['timeout-ms'], 'timeout-ms', DEFAULT_TIMEOUT_MS)
  const outDir = path.resolve(args['out-dir'] || DEFAULT_OUT_DIR)

  assert(typeof token === 'string' && token.trim().length > 0, 'token is required (use --token or WW_TOKEN)')
  assert(projectId.trim().length > 0, 'project-id cannot be empty')

  const report = {
    generatedAt: new Date().toISOString(),
    success: false,
    baseUrl,
    projectId,
    firstTask: null,
    secondTask: null,
    firstConnection: null,
    secondConnection: null,
    assertions: [],
    error: null,
  }

  let firstConnection = null
  let secondConnection = null

  try {
    firstConnection = await createSseConnection({ baseUrl, token, projectId, lastEventId: null })

    const firstEvent = await firstConnection.nextEvent(timeoutMs)
    report.assertions.push(`first connection established via event ${firstEvent.event}`)

    const firstTask = await submitAssetHubTask({ baseUrl, token, suffix: 'first' })
    report.firstTask = firstTask

    const firstLifecycle = await waitForTaskLifecycleEvent({
      connection: firstConnection,
      taskId: firstTask.taskId,
      timeoutMs,
    })
    assert(typeof firstLifecycle.id === 'string' && firstLifecycle.id.trim().length > 0, 'first lifecycle event missing SSE id')

    report.firstConnection = {
      url: firstConnection.url,
      firstEvent,
      lifecycleEvent: firstLifecycle,
      lastEventId: firstLifecycle.id,
    }
    report.assertions.push(`first lifecycle event received for task ${firstTask.taskId} with id=${firstLifecycle.id}`)

    await firstConnection.close()
    firstConnection = null
    report.assertions.push('first connection closed intentionally')

    const secondTask = await submitAssetHubTask({ baseUrl, token, suffix: 'second' })
    report.secondTask = secondTask

    secondConnection = await createSseConnection({
      baseUrl,
      token,
      projectId,
      lastEventId: report.firstConnection.lastEventId,
    })

    const replayed = await waitForTaskLifecycleEvent({
      connection: secondConnection,
      taskId: secondTask.taskId,
      timeoutMs,
    })

    const beforeId = Number(report.firstConnection.lastEventId)
    const replayedId = Number(replayed.id)
    assert(Number.isFinite(beforeId), `last-event-id is not numeric: ${report.firstConnection.lastEventId}`)
    assert(Number.isFinite(replayedId), `replayed event id is not numeric: ${replayed.id}`)
    assert(replayedId > beforeId, `replayed event id ${replayedId} is not greater than last-event-id ${beforeId}`)

    report.secondConnection = {
      url: secondConnection.url,
      replayedEventId: replayed.id,
      replayedEvent: replayed,
    }

    report.assertions.push(`reconnect received task lifecycle event for ${secondTask.taskId}`)
    report.assertions.push(`replayed event id ${replayed.id} > last-event-id ${report.firstConnection.lastEventId}`)

    report.success = true
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    report.error = message
    report.success = false
  } finally {
    if (firstConnection) {
      await firstConnection.close()
    }
    if (secondConnection) {
      await secondConnection.close()
    }
  }

  const { jsonPath, mdPath } = writeReports(outDir, report)
  console.log(`wrote ${jsonPath}`)
  console.log(`wrote ${mdPath}`)
  console.log(`RESULT=${report.success ? 'PASS' : 'FAIL'}`)

  if (!report.success) {
    process.exitCode = 2
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
