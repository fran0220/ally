'use client'

import { useCallback, useMemo } from 'react'
import { useTranslations } from 'next-intl'

import { ProgressToast } from '@/components/ProgressToast'
import { WorkspaceProvider, useWorkspaceProvider } from '@/contexts/WorkspaceProvider'
import {
  WorkspaceStageRuntimeProvider,
  type WorkspaceStageRuntimeValue,
} from '@/contexts/WorkspaceStageRuntimeContext'
import { useUpdateProjectConfig, useUpdateProjectEpisodeField } from '@/lib/query/hooks'

import { StageNavigation } from './StageNavigation'
import ConfigStage from './components/ConfigStage'
import { useWorkspaceExecution } from './hooks/useWorkspaceExecution'
import { useWorkspaceStageNavigation } from './hooks/useWorkspaceStageNavigation'
import type { NovelPromotionWorkspaceProps } from './types'

function StagePlaceholder({ title }: { title: string }) {
  return (
    <div className="glass-surface p-8 text-center">
      <h2 className="text-lg font-semibold text-[var(--glass-text-primary)]">{title}</h2>
      <p className="mt-2 text-sm text-[var(--glass-text-secondary)]">Stage component will be migrated in batch 2.</p>
    </div>
  )
}

function NovelPromotionWorkspaceContent({
  project,
  projectId,
  episodeId,
  episode,
  urlStage,
  onStageChange,
}: NovelPromotionWorkspaceProps) {
  const t = useTranslations('novelPromotion')
  const tc = useTranslations('common')
  const { onRefresh } = useWorkspaceProvider()

  const updateProjectConfigMutation = useUpdateProjectConfig(projectId)
  const updateProjectEpisodeMutation = useUpdateProjectEpisodeField(projectId)

  const currentStage = urlStage === 'text-storyboard' ? 'storyboard' : (urlStage || 'config')
  const projectCharacters = project.novelPromotionData?.characters || []
  const projectLocations = project.novelPromotionData?.locations || []
  const episodeStoryboards = episode?.storyboards || []

  const stageNavItems = useWorkspaceStageNavigation({
    isAnyOperationRunning: false,
    episode,
    projectCharacterCount: projectCharacters.length,
    episodeStoryboards,
    t,
  })

  const handleStageChange = useCallback(
    (stage: string) => {
      onStageChange?.(stage)
    },
    [onStageChange],
  )

  const handleUpdateConfig = useCallback(
    async (key: string, value: unknown) => {
      await updateProjectConfigMutation.mutateAsync({ key, value })
      await onRefresh({ scope: 'project' })
    },
    [onRefresh, updateProjectConfigMutation],
  )

  const handleUpdateEpisode = useCallback(
    async (key: string, value: unknown) => {
      if (!episodeId) return
      await updateProjectEpisodeMutation.mutateAsync({ episodeId, key, value })
      await onRefresh({ scope: 'project' })
    },
    [episodeId, onRefresh, updateProjectEpisodeMutation],
  )

  const execution = useWorkspaceExecution({
    projectId,
    episodeId,
    analysisModel: project.novelPromotionData?.analysisModel || null,
    novelText: episode?.novelText || '',
    t,
    onRefresh,
    onUpdateConfig: handleUpdateConfig,
    onStageChange: handleStageChange,
    onOpenAssetLibrary: () => {
      // Stage 2 will wire the asset library modal.
    },
  })

  const runtimeValue = useMemo<WorkspaceStageRuntimeValue>(
    () => ({
      assetsLoading: false,
      isSubmittingTTS: execution.isSubmittingTTS,
      isTransitioning: execution.isTransitioning,
      isConfirmingAssets: execution.isConfirmingAssets,
      videoRatio: project.novelPromotionData?.videoRatio,
      artStyle: project.novelPromotionData?.artStyle,
      videoModel: project.novelPromotionData?.videoModel,
      capabilityOverrides:
        (project.novelPromotionData?.capabilityOverrides as Record<string, Record<string, string | number | boolean>>) ||
        {},
      userVideoModels: [],
      onNovelTextChange: async (value: string) => {
        await handleUpdateEpisode('novelText', value)
      },
      onVideoRatioChange: async (value: string) => {
        await handleUpdateConfig('videoRatio', value)
      },
      onArtStyleChange: async (value: string) => {
        await handleUpdateConfig('artStyle', value)
      },
      onRunStoryToScript: execution.runStoryToScriptFlow,
      onClipUpdate: async () => {
        // Stage 2 will wire clip editing.
      },
      onOpenAssetLibrary: () => {
        // Stage 2 will wire asset library.
      },
      onRunScriptToStoryboard: execution.runScriptToStoryboardFlow,
      onStageChange: handleStageChange,
      onGenerateVideo: async () => {
        // Stage 2 will wire video generation.
      },
      onGenerateAllVideos: async () => {
        // Stage 2 will wire batch video generation.
      },
      onUpdateVideoPrompt: async () => {
        // Stage 2 will wire prompt editing.
      },
      onUpdatePanelVideoModel: async () => {
        // Stage 2 will wire panel model editing.
      },
      onOpenAssetLibraryForCharacter: () => {
        // Stage 2 will wire focused asset selection.
      },
    }),
    [
      execution.isConfirmingAssets,
      execution.isSubmittingTTS,
      execution.isTransitioning,
      execution.runScriptToStoryboardFlow,
      execution.runStoryToScriptFlow,
      handleStageChange,
      handleUpdateConfig,
      handleUpdateEpisode,
      project.novelPromotionData?.artStyle,
      project.novelPromotionData?.capabilityOverrides,
      project.novelPromotionData?.videoModel,
      project.novelPromotionData?.videoRatio,
    ],
  )

  return (
    <div className="space-y-6">
      <StageNavigation
        projectId={projectId}
        episodeId={episodeId}
        currentStage={currentStage}
        hasNovelText={Boolean((episode?.novelText || '').trim())}
        hasAudio={Boolean(episode?.audioUrl)}
        hasAssets={projectCharacters.length > 0 || projectLocations.length > 0}
        hasStoryboards={episodeStoryboards.some((storyboard) => (storyboard.panels?.length || 0) > 0)}
        hasTextStoryboards={episodeStoryboards.length > 0}
        hasVideos={episodeStoryboards.some((storyboard) =>
          (storyboard.panels || []).some((panel) => Boolean(panel.videoUrl)),
        )}
        hasVoiceLines={(episode?.voiceLines?.length || 0) > 0}
        isDisabled={execution.isTransitioning || execution.isConfirmingAssets}
        onStageClick={handleStageChange}
      />

      <div className="glass-chip glass-chip-neutral inline-flex">{stageNavItems.find((item) => item.id === currentStage)?.label || tc('loading')}</div>

      <WorkspaceStageRuntimeProvider value={runtimeValue}>
        {currentStage === 'config' && <ConfigStage />}
        {currentStage === 'script' && <StagePlaceholder title={t('stages.script')} />}
        {currentStage === 'assets' && <StagePlaceholder title={t('stages.assets')} />}
        {currentStage === 'storyboard' && <StagePlaceholder title={t('stages.storyboard')} />}
        {currentStage === 'videos' && <StagePlaceholder title={t('stages.video')} />}
        {currentStage === 'voice' && <StagePlaceholder title={t('stages.voice')} />}
        {currentStage === 'editor' && <StagePlaceholder title={t('stages.editor')} />}
      </WorkspaceStageRuntimeProvider>

      <ProgressToast show={execution.showCreatingToast} message={t('storyInput.creating')} step={execution.transitionProgress.step} />
    </div>
  )
}

export function NovelPromotionWorkspace(props: NovelPromotionWorkspaceProps) {
  return (
    <WorkspaceProvider projectId={props.projectId} episodeId={props.episodeId}>
      <NovelPromotionWorkspaceContent {...props} />
    </WorkspaceProvider>
  )
}
