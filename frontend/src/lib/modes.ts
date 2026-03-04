import type { ProjectMode } from '@/types/project';

export type { ProjectMode };

export interface ModeConfig {
  id: ProjectMode;
  name: string;
  description: string;
  icon: string;
  color: string;
  available: boolean;
}

export const PROJECT_MODE: ModeConfig = {
  id: 'novel-promotion',
  name: '小说推文',
  description: '从小说生成推广短视频',
  icon: 'N',
  color: 'purple',
  available: true,
};

export const PROJECT_MODES: ModeConfig[] = [PROJECT_MODE];

export function getModeConfig(mode: ProjectMode): ModeConfig | undefined {
  return mode === 'novel-promotion' ? PROJECT_MODE : undefined;
}
