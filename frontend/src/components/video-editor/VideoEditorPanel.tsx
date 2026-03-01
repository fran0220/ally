import { useMemo, useState } from 'react';
import { Player } from '@remotion/player';

import { GlassButton, GlassField, GlassInput, GlassSurface } from '../ui/primitives';

import { Timeline } from './Timeline';
import { VideoComposition } from './VideoComposition';
import { DEFAULT_EDITOR_PROJECT, type TimelineClip, type VideoEditorProjectData } from './types';
import { useVideoEditorState } from './useVideoEditorState';

interface StoryboardHint {
  id: string;
  imageUrl: string | null;
  description: string | null;
}

export interface VideoEditorPanelProps {
  initialData?: VideoEditorProjectData | null;
  storyboardHints?: StoryboardHint[];
  onSave?: (projectData: VideoEditorProjectData) => Promise<void> | void;
  saving?: boolean;
}

function buildClip(kind: TimelineClip['kind'], sourceUrl: string | null, text: string): Omit<TimelineClip, 'id'> {
  return {
    kind,
    sourceUrl,
    text,
    durationInFrames: 90,
    transition: 'fade',
  };
}

export function normalizeEditorProjectData(input: unknown): VideoEditorProjectData {
  if (typeof input !== 'object' || input === null) {
    return DEFAULT_EDITOR_PROJECT;
  }

  const value = input as Partial<VideoEditorProjectData>;
  const timeline = Array.isArray(value.timeline)
    ? value.timeline
        .filter((clip): clip is TimelineClip => typeof clip === 'object' && clip !== null)
        .map((clip) => ({
          ...clip,
          id: clip.id || `legacy-${Math.random().toString(36).slice(2, 8)}`,
          sourceUrl: clip.sourceUrl ?? null,
          text: clip.text ?? '',
          durationInFrames: Math.max(15, Number(clip.durationInFrames) || 90),
          transition: clip.transition ?? 'none',
        }))
    : [];

  return {
    fps: Math.max(1, Number(value.fps) || 30),
    width: Math.max(320, Number(value.width) || 1080),
    height: Math.max(320, Number(value.height) || 1920),
    timeline,
  };
}

export function VideoEditorPanel({ initialData, storyboardHints = [], onSave, saving = false }: VideoEditorPanelProps) {
  const editor = useVideoEditorState(initialData ?? DEFAULT_EDITOR_PROJECT);

  const [sourceUrlInput, setSourceUrlInput] = useState('');
  const [textInput, setTextInput] = useState('');

  const hints = useMemo(
    () => storyboardHints.filter((item) => item.imageUrl).slice(0, 6),
    [storyboardHints],
  );

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
      <GlassSurface>
        <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
          <h3 className="text-sm font-semibold text-[var(--glass-text-secondary)]">Editor Timeline</h3>
          <div className="flex flex-wrap gap-2">
            <GlassButton
              size="sm"
              variant="soft"
              onClick={() => {
                if (!sourceUrlInput.trim()) {
                  return;
                }
                editor.addClip(buildClip('image', sourceUrlInput.trim(), ''));
                setSourceUrlInput('');
              }}
            >
              + Image
            </GlassButton>
            <GlassButton
              size="sm"
              variant="soft"
              onClick={() => {
                if (!sourceUrlInput.trim()) {
                  return;
                }
                editor.addClip(buildClip('video', sourceUrlInput.trim(), ''));
                setSourceUrlInput('');
              }}
            >
              + Video
            </GlassButton>
            <GlassButton
              size="sm"
              variant="soft"
              onClick={() => {
                if (!textInput.trim()) {
                  return;
                }
                editor.addClip(buildClip('text', null, textInput.trim()));
                setTextInput('');
              }}
            >
              + Text
            </GlassButton>
          </div>
        </div>

        <div className="mb-3 grid gap-2 md:grid-cols-2">
          <GlassField label="Media Source URL">
            <GlassInput
              value={sourceUrlInput}
              placeholder="https://..."
              onChange={(event) => setSourceUrlInput(event.target.value)}
            />
          </GlassField>
          <GlassField label="Text Clip Content">
            <GlassInput
              value={textInput}
              placeholder="Narration"
              onChange={(event) => setTextInput(event.target.value)}
            />
          </GlassField>
        </div>

        {hints.length > 0 ? (
          <div className="mb-3 rounded-lg border border-[var(--glass-stroke-base)] bg-[var(--glass-bg-muted)] p-3">
            <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-[var(--glass-text-tertiary)]">Storyboard Hints</p>
            <div className="grid gap-2 sm:grid-cols-2">
              {hints.map((hint) => (
                <button
                  key={hint.id}
                  type="button"
                  className="glass-list-row text-left"
                  onClick={() => {
                    if (hint.imageUrl) {
                      editor.addClip(buildClip('image', hint.imageUrl, hint.description ?? ''));
                    }
                  }}
                >
                  <span className="text-xs font-medium">{hint.description || 'Storyboard image'}</span>
                </button>
              ))}
            </div>
          </div>
        ) : null}

        <Timeline clips={editor.project.timeline} onMove={editor.moveClip} onRemove={editor.removeClip} onUpdate={editor.updateClip} />

        <div className="mt-4 flex flex-wrap items-end gap-2">
          <GlassField label="FPS" className="w-24">
            <GlassInput
              type="number"
              min={1}
              value={String(editor.project.fps)}
              onChange={(event) =>
                editor.setProject({
                  ...editor.project,
                  fps: Math.max(1, Number(event.target.value) || 30),
                })
              }
            />
          </GlassField>
          <GlassField label="Width" className="w-28">
            <GlassInput
              type="number"
              min={320}
              value={String(editor.project.width)}
              onChange={(event) =>
                editor.setProject({
                  ...editor.project,
                  width: Math.max(320, Number(event.target.value) || 1080),
                })
              }
            />
          </GlassField>
          <GlassField label="Height" className="w-28">
            <GlassInput
              type="number"
              min={320}
              value={String(editor.project.height)}
              onChange={(event) =>
                editor.setProject({
                  ...editor.project,
                  height: Math.max(320, Number(event.target.value) || 1920),
                })
              }
            />
          </GlassField>
          <GlassButton
            variant="primary"
            loading={saving}
            disabled={saving || !onSave}
            onClick={() => {
              if (onSave) {
                void onSave(editor.project);
              }
            }}
          >
            Save Editor Data
          </GlassButton>
        </div>
      </GlassSurface>

      <GlassSurface>
        <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">Preview</h3>
        <div className="overflow-hidden rounded-xl border border-[var(--glass-stroke-base)] bg-black">
          <Player
            component={VideoComposition}
            inputProps={{ project: editor.project }}
            durationInFrames={editor.totalFrames}
            fps={editor.project.fps}
            compositionWidth={editor.project.width}
            compositionHeight={editor.project.height}
            controls
            style={{ width: '100%', aspectRatio: `${editor.project.width}/${editor.project.height}` }}
          />
        </div>
      </GlassSurface>
    </div>
  );
}
