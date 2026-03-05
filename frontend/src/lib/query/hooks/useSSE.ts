import { type TaskLifecycleEvent, type TaskStreamEvent } from '@/api/sse'
import { logError as _ulogError } from '@/lib/logging/core'
import { subscribeSharedTaskEvents } from '@/lib/sse/shared-subscriber'
import { isTaskIntent, resolveTaskIntent } from '@/lib/task/intent'
import { TASK_EVENT_TYPE, TASK_SSE_EVENT_TYPE, type SSEEvent } from '@/lib/task/types'

import { useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { queryKeys } from '../keys'
import { applyTaskLifecycleToOverlay } from '../task-target-overlay'

type UseSSEOptions = {
  projectId?: string | null
  episodeId?: string | null
  enabled?: boolean
  onEvent?: (event: SSEEvent) => void
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function toSseEvent(event: TaskLifecycleEvent | TaskStreamEvent): SSEEvent {
  return {
    id: event.id,
    type: event.type,
    taskId: event.taskId,
    projectId: event.projectId,
    userId: event.userId,
    ts: event.ts,
    taskType: event.taskType ?? null,
    targetType: event.targetType ?? null,
    targetId: event.targetId ?? null,
    episodeId: event.episodeId ?? null,
    payload: event.payload as SSEEvent['payload'],
  }
}

export function useSSE({ projectId, episodeId, enabled = true, onEvent }: UseSSEOptions) {
  const queryClient = useQueryClient()
  const [connected, setConnected] = useState(false)
  const targetStatesInvalidateTimerRef = useRef<number | null>(null)
  const isGlobalAssetProject = projectId === 'global-asset-hub'

  useEffect(() => {
    if (!enabled || !projectId) {
      setConnected(false)
      return
    }

    const invalidateEpisodeScoped = (resolvedEpisodeId: string | null) => {
      if (!resolvedEpisodeId) return
      queryClient.invalidateQueries({ queryKey: queryKeys.episodeData(projectId, resolvedEpisodeId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.storyboards.all(resolvedEpisodeId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.voiceLines.all(resolvedEpisodeId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.voiceLines.matched(projectId, resolvedEpisodeId) })
    }

    const invalidateByTarget = (targetType: string | null, resolvedEpisodeId: string | null) => {
      if (isGlobalAssetProject) {
        if (targetType?.startsWith('GlobalCharacter')) {
          queryClient.invalidateQueries({ queryKey: queryKeys.globalAssets.characters() })
          return
        }
        if (targetType?.startsWith('GlobalLocation')) {
          queryClient.invalidateQueries({ queryKey: queryKeys.globalAssets.locations() })
          return
        }
        if (targetType?.startsWith('GlobalVoice')) {
          queryClient.invalidateQueries({ queryKey: queryKeys.globalAssets.voices() })
          return
        }
        queryClient.invalidateQueries({ queryKey: queryKeys.globalAssets.all() })
        return
      }

      if (targetType === 'CharacterAppearance' || targetType === 'NovelPromotionCharacter') {
        queryClient.invalidateQueries({ queryKey: queryKeys.projectAssets.characters(projectId) })
        queryClient.invalidateQueries({ queryKey: queryKeys.projectAssets.all(projectId) })
        return
      }
      if (targetType === 'LocationImage' || targetType === 'NovelPromotionLocation') {
        queryClient.invalidateQueries({ queryKey: queryKeys.projectAssets.locations(projectId) })
        queryClient.invalidateQueries({ queryKey: queryKeys.projectAssets.all(projectId) })
        return
      }
      if (targetType === 'NovelPromotionVoiceLine') {
        invalidateEpisodeScoped(resolvedEpisodeId)
        return
      }
      if (
        targetType === 'NovelPromotionPanel' ||
        targetType === 'NovelPromotionStoryboard' ||
        targetType === 'NovelPromotionShot'
      ) {
        invalidateEpisodeScoped(resolvedEpisodeId)
        return
      }
      if (targetType === 'NovelPromotionEpisode') {
        invalidateEpisodeScoped(resolvedEpisodeId)
        queryClient.invalidateQueries({ queryKey: queryKeys.projectData(projectId) })
        return
      }

      queryClient.invalidateQueries({ queryKey: queryKeys.projectData(projectId) })
    }

    const handleEvent = (event: TaskLifecycleEvent | TaskStreamEvent) => {
      try {
        const payload = toSseEvent(event)
        onEvent?.(payload)

        const eventType = payload.type
        const eventPayload = isRecord(payload.payload) ? payload.payload : null
        const targetType = payload.targetType ?? null
        const targetId = payload.targetId ?? null
        const eventEpisodeId = payload.episodeId ?? null
        const resolvedEpisodeId = eventEpisodeId || episodeId || null
        const lifecycleTypeFromPayload =
          typeof eventPayload?.lifecycleType === 'string'
            ? eventPayload.lifecycleType
            : null

        const rawLifecycleType =
          eventType === TASK_SSE_EVENT_TYPE.LIFECYCLE
            ? lifecycleTypeFromPayload ?? event.eventType
            : null
        const normalizedLifecycleType =
          rawLifecycleType === TASK_EVENT_TYPE.PROGRESS
            ? TASK_EVENT_TYPE.PROCESSING
            : rawLifecycleType
        const isLifecycleEvent = eventType === TASK_SSE_EVENT_TYPE.LIFECYCLE
        const shouldInvalidateTasksList =
          normalizedLifecycleType === TASK_EVENT_TYPE.CREATED ||
          normalizedLifecycleType === TASK_EVENT_TYPE.COMPLETED ||
          normalizedLifecycleType === TASK_EVENT_TYPE.FAILED ||
          (normalizedLifecycleType === TASK_EVENT_TYPE.PROCESSING &&
            typeof eventPayload?.progress !== 'number')
        const shouldInvalidateTargetStates =
          normalizedLifecycleType === TASK_EVENT_TYPE.COMPLETED ||
          normalizedLifecycleType === TASK_EVENT_TYPE.FAILED

        if (isLifecycleEvent && shouldInvalidateTasksList) {
          queryClient.invalidateQueries({ queryKey: queryKeys.tasks.all(projectId) })
        }
        if (isLifecycleEvent && shouldInvalidateTargetStates) {
          if (targetStatesInvalidateTimerRef.current === null) {
            targetStatesInvalidateTimerRef.current = window.setTimeout(() => {
              queryClient.invalidateQueries({ queryKey: queryKeys.tasks.targetStatesAll(projectId), exact: false })
              targetStatesInvalidateTimerRef.current = null
            }, 800)
          }
        }

        const payloadIntent = isTaskIntent(eventPayload?.intent)
          ? eventPayload.intent
          : resolveTaskIntent(typeof payload.taskType === 'string' ? payload.taskType : null)
        const payloadUi =
          eventPayload?.ui && typeof eventPayload.ui === 'object' && !Array.isArray(eventPayload.ui)
            ? (eventPayload.ui as Record<string, unknown>)
            : null
        const hasOutputAtStart =
          typeof payloadUi?.hasOutputAtStart === 'boolean'
            ? payloadUi.hasOutputAtStart
            : null

        applyTaskLifecycleToOverlay(queryClient, {
          projectId,
          lifecycleType: normalizedLifecycleType,
          targetType,
          targetId,
          taskId: payload.taskId || null,
          taskType: payload.taskType ?? null,
          intent: payloadIntent,
          hasOutputAtStart,
          progress: typeof eventPayload?.progress === 'number' ? Math.floor(eventPayload.progress) : null,
          stage: typeof eventPayload?.stage === 'string' ? eventPayload.stage : null,
          stageLabel: typeof eventPayload?.stageLabel === 'string' ? eventPayload.stageLabel : null,
          eventTs: payload.ts || null,
        })

        if (
          normalizedLifecycleType === TASK_EVENT_TYPE.CREATED ||
          normalizedLifecycleType === TASK_EVENT_TYPE.PROCESSING
        ) {
          return
        }

        if (
          normalizedLifecycleType === TASK_EVENT_TYPE.COMPLETED ||
          normalizedLifecycleType === TASK_EVENT_TYPE.FAILED
        ) {
          invalidateByTarget(targetType, resolvedEpisodeId)
        }
      } catch (error) {
        _ulogError('[useSSE] failed to handle event', error)
      }
    }

    setConnected(false)
    const unsubscribe = subscribeSharedTaskEvents({
      projectId,
      episodeId,
      onOpen: () => {
        setConnected(true)
      },
      onError: (error) => {
        setConnected(false)
        _ulogError('[useSSE] stream error', error)
      },
      onLifecycle: (event) => {
        handleEvent(event)
      },
      onStream: (event) => {
        handleEvent(event)
      },
    })

    return () => {
      if (targetStatesInvalidateTimerRef.current !== null) {
        window.clearTimeout(targetStatesInvalidateTimerRef.current)
        targetStatesInvalidateTimerRef.current = null
      }
      unsubscribe()
      setConnected(false)
    }
  }, [enabled, episodeId, projectId, queryClient, isGlobalAssetProject, onEvent])

  return {
    connected,
  }
}
