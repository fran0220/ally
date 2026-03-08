export type StoryboardPhase = 1 | '2-cinematography' | '2-acting' | 3;

export type StoryboardPhaseProgress = {
  start: number;
  end: number;
  labelKey: string;
};

export const PHASE_PROGRESS: Record<string, StoryboardPhaseProgress> = {
  '1': { start: 10, end: 40, labelKey: 'phases.planning' },
  '2-cinematography': { start: 40, end: 55, labelKey: 'phases.cinematography' },
  '2-acting': { start: 55, end: 70, labelKey: 'phases.acting' },
  '3': { start: 70, end: 100, labelKey: 'phases.detail' },
};

export function getStoryboardPhaseProgress(phase: StoryboardPhase): StoryboardPhaseProgress {
  return PHASE_PROGRESS[String(phase)] ?? PHASE_PROGRESS['1']!;
}
