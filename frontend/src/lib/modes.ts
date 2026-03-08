import type { ProjectMode } from '@/types/project';

export type { ProjectMode };

export interface ModeConfig {
  id: ProjectMode;
  nameKey: string;
  descriptionKey: string;
  icon: string;
  color: string;
  available: boolean;
}

export const PROJECT_MODE: ModeConfig = {
  id: 'novel-promotion',
  nameKey: 'modes.novelPromotion.name',
  descriptionKey: 'modes.novelPromotion.description',
  icon: 'N',
  color: 'purple',
  available: true,
};

export const PROJECT_MODES: ModeConfig[] = [PROJECT_MODE];

export function getModeConfig(mode: ProjectMode): ModeConfig | undefined {
  return mode === 'novel-promotion' ? PROJECT_MODE : undefined;
}
