import { GlassButton, GlassInput } from '../ui/primitives';

import { TransitionPicker } from './TransitionPicker';
import type { TimelineClip } from './types';

interface TimelineProps {
  clips: TimelineClip[];
  onMove: (clipId: string, direction: 'up' | 'down') => void;
  onRemove: (clipId: string) => void;
  onUpdate: (clipId: string, patch: Partial<TimelineClip>) => void;
}

export function Timeline({ clips, onMove, onRemove, onUpdate }: TimelineProps) {
  return (
    <div className="space-y-2">
      {clips.map((clip, index) => (
        <article key={clip.id} className="glass-list-row flex-col items-stretch gap-2 p-3">
          <div className="flex items-center justify-between gap-2">
            <div>
              <p className="text-sm font-semibold text-[var(--glass-text-primary)]">
                #{index + 1} · {clip.kind.toUpperCase()}
              </p>
              <p className="text-xs text-[var(--glass-text-tertiary)]">{clip.sourceUrl || clip.text || 'No source'}</p>
            </div>
            <div className="flex gap-1">
              <GlassButton
                type="button"
                size="sm"
                variant="soft"
                onClick={() => onMove(clip.id, 'up')}
                disabled={index === 0}
              >
                ↑
              </GlassButton>
              <GlassButton
                type="button"
                size="sm"
                variant="soft"
                onClick={() => onMove(clip.id, 'down')}
                disabled={index === clips.length - 1}
              >
                ↓
              </GlassButton>
              <GlassButton type="button" size="sm" variant="danger" onClick={() => onRemove(clip.id)}>
                Delete
              </GlassButton>
            </div>
          </div>

          <div className="grid gap-2 sm:grid-cols-3">
            <label className="text-xs text-[var(--glass-text-secondary)]">
              Duration (frames)
              <GlassInput
                className="mt-1"
                type="number"
                min={15}
                value={String(clip.durationInFrames)}
                onChange={(event) =>
                  onUpdate(clip.id, {
                    durationInFrames: Math.max(15, Number(event.target.value) || 15),
                  })
                }
              />
            </label>
            <label className="text-xs text-[var(--glass-text-secondary)]">
              Transition
              <div className="mt-1">
                <TransitionPicker value={clip.transition} onChange={(transition) => onUpdate(clip.id, { transition })} />
              </div>
            </label>
            {clip.kind === 'text' ? (
              <label className="text-xs text-[var(--glass-text-secondary)]">
                Text
                <GlassInput
                  className="mt-1"
                  value={clip.text}
                  onChange={(event) => onUpdate(clip.id, { text: event.target.value })}
                />
              </label>
            ) : (
              <label className="text-xs text-[var(--glass-text-secondary)]">
                Source URL
                <GlassInput
                  className="mt-1"
                  value={clip.sourceUrl ?? ''}
                  onChange={(event) => onUpdate(clip.id, { sourceUrl: event.target.value })}
                />
              </label>
            )}
          </div>
        </article>
      ))}
      {clips.length === 0 ? <p className="text-sm text-[var(--glass-text-tertiary)]">No clips yet.</p> : null}
    </div>
  );
}
