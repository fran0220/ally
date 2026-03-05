import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Link, useParams, useSearchParams } from 'react-router-dom';

import {
  createEpisode,
  downloadProjectImagesZip,
  downloadProjectVideosZip,
  downloadProjectVoicesZip,
  deleteEpisode,
  getEditorProject,
  getEpisode,
  getNovelProject,
  listStoryboards,
  listVideoUrls,
  listVoiceLines,
  saveEditorProject,
  splitEpisodesByMarkers,
  updateEpisode,
} from '../../api/novel';
import { listTasks } from '../../api/tasks';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { ProgressToast } from '../../components/ProgressToast';
import { MediaImageWithLoading } from '../../components/media';
import { ImagePreviewModal } from '../../components/ui/ImagePreviewModal';
import { VideoEditorPanel, normalizeEditorProjectData, type VideoEditorProjectData } from '../../components/video-editor';
import {
  GlassButton,
  GlassChip,
  GlassField,
  GlassInput,
  GlassModalShell,
  GlassSurface,
  GlassTextarea,
} from '../../components/ui/primitives';
import { useTaskSse } from '../../hooks/use-task-sse';
import { queryKeys } from '../../lib/query-keys';

const STAGES = [
  'config',
  'script',
  'assets',
  'prompts',
  'text-storyboard',
  'storyboard',
  'videos',
  'voice',
  'editor',
] as const;

type StageId = (typeof STAGES)[number];

function isStage(value: string | null): value is StageId {
  if (!value) {
    return false;
  }
  return STAGES.includes(value as StageId);
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function readString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === 'string' ? value : null;
}

function readNumber(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  return typeof value === 'number' ? value : null;
}

function stageLabel(stage: StageId): string {
  if (stage === 'prompts') {
    return '4. Prompt Workshop';
  }
  if (stage === 'text-storyboard') {
    return '5. Text Storyboard';
  }
  if (stage === 'editor') {
    return '9. Video Editor';
  }
  if (stage === 'script') {
    return '2. Script Draft';
  }
  if (stage === 'assets') {
    return '3. Asset Analysis';
  }
  if (stage === 'storyboard') {
    return '6. Storyboard Review';
  }
  if (stage === 'videos') {
    return '7. Video Generation';
  }
  if (stage === 'voice') {
    return '8. Voice Stage';
  }
  return '1. Config';
}

function triggerZipDownload(blob: Blob, fileName: string): void {
  const objectUrl = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = objectUrl;
  anchor.download = fileName;
  anchor.click();
  setTimeout(() => {
    URL.revokeObjectURL(objectUrl);
  }, 500);
}

export function ProjectWorkbench() {
  const { t } = useTranslation(['workspaceDetail', 'common']);
  const queryClient = useQueryClient();
  const { projectId } = useParams<{ projectId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();

  const [episodeModalOpen, setEpisodeModalOpen] = useState(false);
  const [newEpisodeName, setNewEpisodeName] = useState('');
  const [episodeDraftText, setEpisodeDraftText] = useState('');
  const [renameEpisodeId, setRenameEpisodeId] = useState<string | null>(null);
  const [renameEpisodeName, setRenameEpisodeName] = useState('');
  const [deleteEpisodeTarget, setDeleteEpisodeTarget] = useState<{ id: string; name: string } | null>(null);
  const [previewImageUrl, setPreviewImageUrl] = useState<string | null>(null);
  const [smartImportText, setSmartImportText] = useState('');
  const [smartImportError, setSmartImportError] = useState<string | null>(null);

  if (!projectId) {
    return (
      <main className="page-shell py-10">
        <GlassSurface>Project id is missing.</GlassSurface>
      </main>
    );
  }

  const rootQuery = useQuery({
    queryKey: queryKeys.novel.root(projectId),
    queryFn: () => getNovelProject(projectId),
  });

  const episodes = rootQuery.data?.project.novelPromotionData.episodes ?? [];
  const stageParam = searchParams.get('stage');
  const currentStage: StageId = isStage(stageParam) ? stageParam : 'config';

  const episodeParam = searchParams.get('episode');
  const selectedEpisodeId = episodes.find((episode) => episode.id === episodeParam)?.id ?? episodes[0]?.id ?? null;

  useEffect(() => {
    if (!episodes.length) {
      return;
    }

    const next = new URLSearchParams(searchParams);
    let changed = false;
    if (!isStage(stageParam)) {
      next.set('stage', 'config');
      changed = true;
    }
    if (!selectedEpisodeId) {
      return;
    }
    if (next.get('episode') !== selectedEpisodeId) {
      next.set('episode', selectedEpisodeId);
      changed = true;
    }
    if (changed) {
      setSearchParams(next, { replace: true });
    }
  }, [episodes, searchParams, selectedEpisodeId, setSearchParams, stageParam]);

  const episodeQuery = useQuery({
    queryKey: selectedEpisodeId ? queryKeys.novel.episode(projectId, selectedEpisodeId) : ['novel', 'episode', projectId, 'none'],
    queryFn: () => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return getEpisode(projectId, selectedEpisodeId);
    },
    enabled: selectedEpisodeId !== null,
  });

  const storyboardsQuery = useQuery({
    queryKey: selectedEpisodeId ? queryKeys.novel.storyboards(projectId, selectedEpisodeId) : ['novel', 'storyboards', projectId, 'none'],
    queryFn: () => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return listStoryboards(projectId, selectedEpisodeId);
    },
    enabled: selectedEpisodeId !== null,
  });

  const videoUrlsQuery = useQuery({
    queryKey: selectedEpisodeId ? ['novel', 'video-urls', projectId, selectedEpisodeId] : ['novel', 'video-urls', projectId, 'none'],
    queryFn: () => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return listVideoUrls(projectId, selectedEpisodeId);
    },
    enabled: selectedEpisodeId !== null,
  });

  const voiceLinesQuery = useQuery({
    queryKey: selectedEpisodeId ? ['novel', 'voice-lines', projectId, selectedEpisodeId] : ['novel', 'voice-lines', projectId, 'none'],
    queryFn: () => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return listVoiceLines(projectId, selectedEpisodeId);
    },
    enabled: selectedEpisodeId !== null,
  });

  const editorQuery = useQuery({
    queryKey: selectedEpisodeId ? queryKeys.novel.editor(projectId, selectedEpisodeId) : ['novel', 'editor', projectId, 'none'],
    queryFn: () => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return getEditorProject(projectId, selectedEpisodeId);
    },
    enabled: selectedEpisodeId !== null,
  });

  const tasksQuery = useQuery({
    queryKey: queryKeys.tasks.list(projectId, selectedEpisodeId),
    queryFn: () => listTasks({ projectId, limit: 100 }),
  });

  const episodeCreateMutation = useMutation({
    mutationFn: ({ name, novelText }: { name: string; novelText: string }) =>
      createEpisode(projectId, { name, novelText }),
    onSuccess: () => {
      setEpisodeModalOpen(false);
      setNewEpisodeName('');
      setEpisodeDraftText('');
      void queryClient.invalidateQueries({ queryKey: queryKeys.novel.root(projectId) });
    },
  });

  const episodeRenameMutation = useMutation({
    mutationFn: ({ episodeId, name }: { episodeId: string; name: string }) =>
      updateEpisode(projectId, episodeId, { name }),
    onSuccess: () => {
      setRenameEpisodeId(null);
      setRenameEpisodeName('');
      void queryClient.invalidateQueries({ queryKey: queryKeys.novel.root(projectId) });
    },
  });

  const episodeDeleteMutation = useMutation({
    mutationFn: (episodeId: string) => deleteEpisode(projectId, episodeId),
    onSuccess: () => {
      setDeleteEpisodeTarget(null);
      void queryClient.invalidateQueries({ queryKey: queryKeys.novel.root(projectId) });
    },
  });

  const editorSaveMutation = useMutation({
    mutationFn: (projectData: VideoEditorProjectData) => {
      if (!selectedEpisodeId) {
        throw new Error('Episode id is required');
      }
      return saveEditorProject(projectId, selectedEpisodeId, projectData as unknown as Record<string, unknown>);
    },
    onSuccess: () => {
      if (selectedEpisodeId) {
        void queryClient.invalidateQueries({ queryKey: queryKeys.novel.editor(projectId, selectedEpisodeId) });
      }
    },
  });

  const smartImportMutation = useMutation({
    mutationFn: (content: string) => splitEpisodesByMarkers(projectId, content),
    onSuccess: async (result) => {
      setSmartImportError(null);
      setSmartImportText('');
      await queryClient.invalidateQueries({ queryKey: queryKeys.novel.root(projectId) });
      const firstEpisodeId = result.episodes?.[0]?.id ?? null;
      if (firstEpisodeId) {
        updateUrl({ stage: 'config', episode: firstEpisodeId });
      }
    },
    onError: (error) => {
      setSmartImportError(error instanceof Error ? error.message : 'Smart import failed.');
    },
  });

  const downloadImagesMutation = useMutation({
    mutationFn: () => downloadProjectImagesZip(projectId, selectedEpisodeId ?? undefined),
    onSuccess: (blob) => triggerZipDownload(blob, `${projectId}_images.zip`),
  });

  const downloadVideosMutation = useMutation({
    mutationFn: () => downloadProjectVideosZip(projectId, { episodeId: selectedEpisodeId ?? undefined }),
    onSuccess: (blob) => triggerZipDownload(blob, `${projectId}_videos.zip`),
  });

  const downloadVoicesMutation = useMutation({
    mutationFn: () => downloadProjectVoicesZip(projectId, selectedEpisodeId ?? undefined),
    onSuccess: (blob) => triggerZipDownload(blob, `${projectId}_voices.zip`),
  });

  const { connected, events } = useTaskSse({ projectId, episodeId: selectedEpisodeId, enabled: true });

  const selectedEpisode = episodeQuery.data?.episode;
  const activeTasks = useMemo(() => {
    const all = tasksQuery.data?.tasks ?? [];
    if (!selectedEpisodeId) {
      return all;
    }
    return all.filter((task) => task.episodeId === selectedEpisodeId || task.episodeId === null);
  }, [tasksQuery.data?.tasks, selectedEpisodeId]);

  const storyboardHints = useMemo(() => {
    const rows = storyboardsQuery.data?.storyboards ?? [];
    return rows
      .map((item) => {
        const row = asRecord(item);
        if (!row) {
          return null;
        }
        return {
          id: readString(row, 'id') ?? `storyboard-${Math.random().toString(36).slice(2, 7)}`,
          imageUrl: readString(row, 'storyboardImageUrl'),
          description: readString(row, 'clipId'),
        };
      })
      .filter((item): item is { id: string; imageUrl: string | null; description: string | null } => item !== null);
  }, [storyboardsQuery.data?.storyboards]);

  const editorData = useMemo(
    () => normalizeEditorProjectData(editorQuery.data?.projectData),
    [editorQuery.data?.projectData],
  );
  const showMutationProgress =
    episodeCreateMutation.isPending || episodeRenameMutation.isPending || episodeDeleteMutation.isPending;

  function updateUrl(next: Partial<{ stage: StageId; episode: string | null }>) {
    const params = new URLSearchParams(searchParams);
    if (next.stage) {
      params.set('stage', next.stage);
    }
    if (next.episode !== undefined) {
      if (next.episode) {
        params.set('episode', next.episode);
      } else {
        params.delete('episode');
      }
    }
    setSearchParams(params, { replace: true });
  }

  return (
    <main className="page-shell py-8 md:py-10">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="glass-page-title">{rootQuery.data?.project.name ?? projectId}</h1>
          <p className="glass-page-subtitle">
            {t('workspaceDetail:episode')}: {selectedEpisode?.name ?? 'N/A'}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <GlassChip tone={connected ? 'success' : 'warning'}>{connected ? 'SSE Live' : 'SSE Reconnecting'}</GlassChip>
          <GlassChip tone="neutral">{activeTasks.length} tasks</GlassChip>
          <Link to="/workspace" className="glass-btn-base glass-btn-soft h-9 px-3 text-sm">
            {t('workspaceDetail:backToWorkspace')}
          </Link>
        </div>
      </header>

      {rootQuery.error instanceof Error ? (
        <GlassSurface className="mb-4">
          <p className="text-sm text-[var(--glass-tone-danger-fg)]">{rootQuery.error.message}</p>
        </GlassSurface>
      ) : null}

      {episodes.length === 0 ? (
        <GlassSurface className="space-y-4 p-6">
          <h2 className="text-lg font-semibold">Smart Import</h2>
          <p className="text-sm text-[var(--glass-text-secondary)]">
            Paste full novel content and split episodes automatically, or create the first episode manually.
          </p>
          <GlassTextarea
            rows={12}
            value={smartImportText}
            onChange={(event) => setSmartImportText(event.target.value)}
            placeholder="Paste the full story text here..."
          />
          {smartImportError ? (
            <p className="text-sm text-[var(--glass-tone-danger-fg)]">{smartImportError}</p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            <GlassButton
              variant="primary"
              loading={smartImportMutation.isPending}
              onClick={() => {
                const trimmed = smartImportText.trim();
                if (!trimmed) {
                  setSmartImportError('Please paste content before importing.');
                  return;
                }
                smartImportMutation.mutate(trimmed);
              }}
            >
              Split by Markers
            </GlassButton>
            <GlassButton
              variant="soft"
              loading={episodeCreateMutation.isPending}
              onClick={() => {
                episodeCreateMutation.mutate({
                  name: 'Episode 1',
                  novelText: '',
                });
              }}
            >
              Manual Create Episode
            </GlassButton>
          </div>
        </GlassSurface>
      ) : null}

      {episodes.length > 0 ? (
      <div className="grid gap-4 lg:grid-cols-[240px_minmax(0,1fr)]">
        <GlassSurface>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Episodes</h2>
            <GlassButton size="sm" variant="soft" onClick={() => setEpisodeModalOpen(true)}>
              +
            </GlassButton>
          </div>
          <div className="space-y-2">
            {episodes.map((episode) => (
              <article
                key={episode.id}
                className={`glass-list-row flex-col items-stretch gap-2 p-2 ${
                  selectedEpisodeId === episode.id ? 'border-[var(--glass-stroke-focus)]' : ''
                }`}
              >
                <button
                  type="button"
                  className="text-left"
                  onClick={() => {
                    updateUrl({ episode: episode.id });
                  }}
                >
                  <p className="text-sm font-semibold">{episode.name}</p>
                  <p className="text-xs text-[var(--glass-text-tertiary)]">#{episode.episodeNumber}</p>
                </button>
                <div className="flex gap-1">
                  <GlassButton
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setRenameEpisodeId(episode.id);
                      setRenameEpisodeName(episode.name);
                    }}
                  >
                    {t('common:edit')}
                  </GlassButton>
                  <GlassButton
                    size="sm"
                    variant="danger"
                    onClick={() => {
                      setDeleteEpisodeTarget({ id: episode.id, name: episode.name });
                    }}
                  >
                    {t('common:delete')}
                  </GlassButton>
                </div>
              </article>
            ))}
            {episodes.length === 0 ? <p className="text-xs text-[var(--glass-text-tertiary)]">No episodes.</p> : null}
          </div>
        </GlassSurface>

        <div className="space-y-4">
          <GlassSurface>
            <div
              className="flex flex-nowrap gap-2 overflow-x-auto pb-1 [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden md:flex-wrap md:overflow-visible md:pb-0"
              style={{ WebkitOverflowScrolling: 'touch' }}
            >
              {STAGES.map((stage) => (
                <GlassButton
                  key={stage}
                  size="sm"
                  className="shrink-0 whitespace-nowrap"
                  variant={stage === currentStage ? 'primary' : 'soft'}
                  onClick={() => updateUrl({ stage })}
                >
                  {stageLabel(stage)}
                </GlassButton>
              ))}
            </div>
          </GlassSurface>

          {currentStage === 'config' ? (
            <GlassSurface>
              <h3 className="mb-3 text-base font-semibold">Stage: Config</h3>
              <dl className="grid gap-3 sm:grid-cols-2">
                <div className="glass-kpi p-3">
                  <dt className="text-xs text-[var(--glass-text-tertiary)]">Analysis Model</dt>
                  <dd className="text-sm font-medium">{rootQuery.data?.project.novelPromotionData.analysisModel ?? 'N/A'}</dd>
                </div>
                <div className="glass-kpi p-3">
                  <dt className="text-xs text-[var(--glass-text-tertiary)]">Image Model</dt>
                  <dd className="text-sm font-medium">{rootQuery.data?.project.novelPromotionData.imageModel ?? 'N/A'}</dd>
                </div>
                <div className="glass-kpi p-3">
                  <dt className="text-xs text-[var(--glass-text-tertiary)]">Video Ratio</dt>
                  <dd className="text-sm font-medium">{rootQuery.data?.project.novelPromotionData.videoRatio ?? 'N/A'}</dd>
                </div>
                <div className="glass-kpi p-3">
                  <dt className="text-xs text-[var(--glass-text-tertiary)]">TTS Rate</dt>
                  <dd className="text-sm font-medium">{rootQuery.data?.project.novelPromotionData.ttsRate ?? 'N/A'}</dd>
                </div>
              </dl>
            </GlassSurface>
          ) : null}

          {currentStage === 'script' ? (
            <GlassSurface>
              <h3 className="mb-3 text-base font-semibold">Stage: Script</h3>
              <GlassTextarea rows={12} value={selectedEpisode?.novelText ?? ''} readOnly />
            </GlassSurface>
          ) : null}

          {currentStage === 'assets' ? (
            <GlassSurface>
              <h3 className="mb-3 text-base font-semibold">Stage: Assets</h3>
              <div className="grid gap-3 md:grid-cols-2">
                <div className="glass-kpi p-3">
                  <p className="text-xs text-[var(--glass-text-tertiary)]">Characters</p>
                  <p className="text-2xl font-semibold">{rootQuery.data?.project.novelPromotionData.characters.length ?? 0}</p>
                </div>
                <div className="glass-kpi p-3">
                  <p className="text-xs text-[var(--glass-text-tertiary)]">Locations</p>
                  <p className="text-2xl font-semibold">{rootQuery.data?.project.novelPromotionData.locations.length ?? 0}</p>
                </div>
              </div>
            </GlassSurface>
          ) : null}

          {currentStage === 'prompts' ? (
            <GlassSurface>
              <h3 className="mb-3 text-base font-semibold">Stage: Prompts</h3>
              <p className="text-sm text-[var(--glass-text-secondary)]">
                Global prompt context:
                <span className="ml-1 font-medium text-[var(--glass-text-primary)]">
                  {rootQuery.data?.project.novelPromotionData.globalAssetText || 'Not set'}
                </span>
              </p>
            </GlassSurface>
          ) : null}

          {currentStage === 'text-storyboard' ? (
            <GlassSurface>
              <h3 className="mb-3 text-base font-semibold">Stage: Text Storyboard</h3>
              <p className="text-sm text-[var(--glass-text-secondary)]">
                Storyboard groups: {storyboardsQuery.data?.storyboards.length ?? 0}
              </p>
            </GlassSurface>
          ) : null}

          {currentStage === 'storyboard' ? (
            <GlassSurface>
              <div className="mb-3 flex items-center justify-between gap-2">
                <h3 className="text-base font-semibold">Stage: Storyboard</h3>
                <GlassButton
                  size="sm"
                  variant="soft"
                  loading={downloadImagesMutation.isPending}
                  onClick={() => {
                    downloadImagesMutation.mutate();
                  }}
                >
                  Download Images ZIP
                </GlassButton>
              </div>
              <div className="space-y-2">
                {(storyboardsQuery.data?.storyboards ?? []).map((item) => {
                  const row = asRecord(item);
                  if (!row) {
                    return null;
                  }
                  return (
                    <article key={readString(row, 'id') ?? String(Math.random())} className="glass-list-row">
                      <span className="text-sm font-medium">{readString(row, 'id') ?? 'unknown'}</span>
                      <span className="text-xs text-[var(--glass-text-tertiary)]">
                        panels: {readNumber(row, 'panelCount') ?? 0}
                      </span>
                    </article>
                  );
                })}
              </div>

              <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {storyboardHints.map((item) => {
                  if (!item.imageUrl) {
                    return null;
                  }
                  return (
                    <button
                      key={`preview-${item.id}`}
                      type="button"
                      className="glass-list-row flex-col items-stretch gap-2 p-2 text-left"
                      onClick={() => setPreviewImageUrl(item.imageUrl)}
                    >
                      <MediaImageWithLoading
                        src={item.imageUrl}
                        alt={item.description ?? item.id}
                        containerClassName="h-32 rounded-lg"
                        className="h-32 w-full object-cover"
                      />
                      <span className="truncate text-xs text-[var(--glass-text-secondary)]">
                        {item.description ?? item.id}
                      </span>
                    </button>
                  );
                })}
              </div>
            </GlassSurface>
          ) : null}

          {currentStage === 'videos' ? (
            <GlassSurface>
              <div className="mb-3 flex items-center justify-between gap-2">
                <h3 className="text-base font-semibold">Stage: Videos</h3>
                <GlassButton
                  size="sm"
                  variant="soft"
                  loading={downloadVideosMutation.isPending}
                  onClick={() => {
                    downloadVideosMutation.mutate();
                  }}
                >
                  Download Videos ZIP
                </GlassButton>
              </div>
              <div className="space-y-2">
                {(videoUrlsQuery.data?.videos ?? []).map((video) => (
                  <article key={video.panelId} className="glass-list-row">
                    <span className="text-sm">Panel #{video.panelIndex + 1}</span>
                    <a className="text-xs text-[var(--glass-tone-info-fg)] underline" href={video.videoUrl} target="_blank" rel="noreferrer">
                      Preview
                    </a>
                  </article>
                ))}
                {(videoUrlsQuery.data?.videos ?? []).length === 0 ? (
                  <p className="text-sm text-[var(--glass-text-tertiary)]">No generated videos yet.</p>
                ) : null}
              </div>
            </GlassSurface>
          ) : null}

          {currentStage === 'voice' ? (
            <GlassSurface>
              <div className="mb-3 flex items-center justify-between gap-2">
                <h3 className="text-base font-semibold">Stage: Voice</h3>
                <GlassButton
                  size="sm"
                  variant="soft"
                  loading={downloadVoicesMutation.isPending}
                  onClick={() => {
                    downloadVoicesMutation.mutate();
                  }}
                >
                  Download Voices ZIP
                </GlassButton>
              </div>
              <div className="space-y-2">
                {(voiceLinesQuery.data?.voiceLines ?? []).map((line) => (
                  <article key={line.id} className="glass-list-row flex-col items-stretch gap-1">
                    <p className="text-xs text-[var(--glass-text-tertiary)]">
                      #{line.lineIndex} · {line.speaker}
                    </p>
                    <p className="text-sm">{line.content}</p>
                    {line.audioUrl ? (
                      <audio className="w-full" controls src={line.audioUrl} />
                    ) : (
                      <p className="text-xs text-[var(--glass-text-tertiary)]">No audio yet</p>
                    )}
                  </article>
                ))}
                {(voiceLinesQuery.data?.voiceLines ?? []).length === 0 ? (
                  <p className="text-sm text-[var(--glass-text-tertiary)]">No voice lines generated.</p>
                ) : null}
              </div>
            </GlassSurface>
          ) : null}

          {currentStage === 'editor' ? (
            <GlassSurface>
              <h3 className="mb-4 text-base font-semibold">Stage: Editor (Remotion)</h3>
              <VideoEditorPanel
                key={`${selectedEpisodeId ?? 'none'}-${editorQuery.data?.updatedAt ?? 'new'}`}
                initialData={editorData}
                storyboardHints={storyboardHints}
                saving={editorSaveMutation.isPending}
                onSave={async (projectData) => {
                  await editorSaveMutation.mutateAsync(projectData);
                }}
              />
            </GlassSurface>
          ) : null}

          <GlassSurface>
            <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">Realtime Events</h3>
            <div className="space-y-2">
              {events.slice(0, 8).map((event) => (
                <article key={`${event.id}-${event.ts}`} className="glass-list-row flex-col items-stretch gap-1 p-2">
                  <p className="text-xs text-[var(--glass-text-tertiary)]">{new Date(event.ts).toLocaleTimeString()}</p>
                  <p className="text-sm font-medium">{event.eventType}</p>
                  <p className="text-xs text-[var(--glass-text-secondary)]">
                    {event.taskType ?? 'unknown'} → {event.targetType ?? 'unknown'}
                  </p>
                </article>
              ))}
              {events.length === 0 ? <p className="text-xs text-[var(--glass-text-tertiary)]">No realtime events yet.</p> : null}
            </div>
          </GlassSurface>
        </div>
      </div>
      ) : null}

      <GlassModalShell
        open={episodeModalOpen}
        onClose={() => setEpisodeModalOpen(false)}
        title="Create Episode"
      >
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!newEpisodeName.trim()) {
              return;
            }
            episodeCreateMutation.mutate({
              name: newEpisodeName.trim(),
              novelText: episodeDraftText,
            });
          }}
        >
          <GlassField label="Episode Name" required>
            <GlassInput value={newEpisodeName} onChange={(event) => setNewEpisodeName(event.target.value)} />
          </GlassField>
          <GlassField label="Draft Novel Text">
            <GlassTextarea rows={5} value={episodeDraftText} onChange={(event) => setEpisodeDraftText(event.target.value)} />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setEpisodeModalOpen(false)}>
              {t('common:cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={episodeCreateMutation.isPending}>
              Create
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell
        open={renameEpisodeId !== null}
        onClose={() => setRenameEpisodeId(null)}
        title="Rename Episode"
        size="sm"
      >
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!renameEpisodeId || !renameEpisodeName.trim()) {
              return;
            }
            episodeRenameMutation.mutate({
              episodeId: renameEpisodeId,
              name: renameEpisodeName.trim(),
            });
          }}
        >
          <GlassField label="Episode Name" required>
            <GlassInput value={renameEpisodeName} onChange={(event) => setRenameEpisodeName(event.target.value)} />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setRenameEpisodeId(null)}>
              {t('common:cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={episodeRenameMutation.isPending}>
              {t('common:save')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <ConfirmDialog
        show={deleteEpisodeTarget !== null}
        title={t('common:deleteEpisodeConfirm')}
        message={deleteEpisodeTarget ? `${t('common:deleteEpisode')}: ${deleteEpisodeTarget.name}` : ''}
        confirmText={t('common:delete')}
        cancelText={t('common:cancel')}
        onCancel={() => setDeleteEpisodeTarget(null)}
        onConfirm={() => {
          if (!deleteEpisodeTarget) {
            return;
          }
          episodeDeleteMutation.mutate(deleteEpisodeTarget.id);
        }}
        type="danger"
      />

      <ImagePreviewModal imageUrl={previewImageUrl} onClose={() => setPreviewImageUrl(null)} />

      <ProgressToast
        show={showMutationProgress}
        message={t('common:loading')}
        step={t('workspaceDetail:episode')}
      />
    </main>
  );
}
