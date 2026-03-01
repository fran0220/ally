export type ClipKind = 'image' | 'video' | 'text';
export type ClipTransition = 'none' | 'fade' | 'slide-left' | 'zoom-in';

export interface TimelineClip {
  id: string;
  kind: ClipKind;
  sourceUrl: string | null;
  text: string;
  durationInFrames: number;
  transition: ClipTransition;
}

export interface VideoEditorProjectData {
  fps: number;
  width: number;
  height: number;
  timeline: TimelineClip[];
}

export const DEFAULT_EDITOR_PROJECT: VideoEditorProjectData = {
  fps: 30,
  width: 1080,
  height: 1920,
  timeline: [],
};
