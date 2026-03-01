import { useMemo, useState } from 'react';

import { DEFAULT_EDITOR_PROJECT, type TimelineClip, type VideoEditorProjectData } from './types';

function randomId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

export interface VideoEditorState {
  project: VideoEditorProjectData;
  totalFrames: number;
  setProject: (next: VideoEditorProjectData) => void;
  addClip: (clip: Omit<TimelineClip, 'id'>) => void;
  removeClip: (clipId: string) => void;
  moveClip: (clipId: string, direction: 'up' | 'down') => void;
  updateClip: (clipId: string, patch: Partial<TimelineClip>) => void;
}

export function useVideoEditorState(initialData?: VideoEditorProjectData | null): VideoEditorState {
  const [project, setProject] = useState<VideoEditorProjectData>(initialData ?? DEFAULT_EDITOR_PROJECT);

  const totalFrames = useMemo(
    () => Math.max(1, project.timeline.reduce((sum, clip) => sum + clip.durationInFrames, 0)),
    [project.timeline],
  );

  const addClip = (clip: Omit<TimelineClip, 'id'>) => {
    setProject((previous) => ({
      ...previous,
      timeline: [...previous.timeline, { ...clip, id: randomId('clip') }],
    }));
  };

  const removeClip = (clipId: string) => {
    setProject((previous) => ({
      ...previous,
      timeline: previous.timeline.filter((clip) => clip.id !== clipId),
    }));
  };

  const moveClip = (clipId: string, direction: 'up' | 'down') => {
    setProject((previous) => {
      const index = previous.timeline.findIndex((clip) => clip.id === clipId);
      if (index === -1) {
        return previous;
      }

      const swapIndex = direction === 'up' ? index - 1 : index + 1;
      if (swapIndex < 0 || swapIndex >= previous.timeline.length) {
        return previous;
      }

      const next = [...previous.timeline];
      const sourceClip = next[index];
      const targetClip = next[swapIndex];
      if (!sourceClip || !targetClip) {
        return previous;
      }

      next[index] = targetClip;
      next[swapIndex] = sourceClip;
      return { ...previous, timeline: next };
    });
  };

  const updateClip = (clipId: string, patch: Partial<TimelineClip>) => {
    setProject((previous) => ({
      ...previous,
      timeline: previous.timeline.map((clip) => (clip.id === clipId ? { ...clip, ...patch } : clip)),
    }));
  };

  return {
    project,
    totalFrames,
    setProject,
    addClip,
    removeClip,
    moveClip,
    updateClip,
  };
}
