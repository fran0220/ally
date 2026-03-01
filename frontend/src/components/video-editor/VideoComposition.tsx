import { AbsoluteFill, Img, Sequence, Video, interpolate, useCurrentFrame } from 'remotion';

import type { TimelineClip, VideoEditorProjectData } from './types';

interface VideoCompositionProps {
  project: VideoEditorProjectData;
}

function transitionStyle(clip: TimelineClip, relativeFrame: number, durationInFrames: number) {
  const introFrames = Math.min(12, Math.floor(durationInFrames / 3));

  if (clip.transition === 'fade') {
    return {
      opacity: interpolate(relativeFrame, [0, introFrames], [0, 1], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
      }),
    };
  }

  if (clip.transition === 'slide-left') {
    return {
      transform: `translateX(${interpolate(relativeFrame, [0, introFrames], [40, 0], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
      })}px)`,
      opacity: interpolate(relativeFrame, [0, introFrames], [0.2, 1], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
      }),
    };
  }

  if (clip.transition === 'zoom-in') {
    return {
      transform: `scale(${interpolate(relativeFrame, [0, introFrames], [1.12, 1], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
      })})`,
      opacity: interpolate(relativeFrame, [0, introFrames], [0.4, 1], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
      }),
    };
  }

  return {};
}

export function VideoComposition({ project }: VideoCompositionProps) {
  const frame = useCurrentFrame();

  let offset = 0;
  return (
    <AbsoluteFill style={{ backgroundColor: '#020817' }}>
      {project.timeline.map((clip) => {
        const from = offset;
        offset += clip.durationInFrames;

        const style = transitionStyle(clip, frame - from, clip.durationInFrames);
        const commonStyle = {
          width: '100%',
          height: '100%',
          objectFit: 'cover' as const,
          ...style,
        };

        return (
          <Sequence key={clip.id} from={from} durationInFrames={clip.durationInFrames}>
            <AbsoluteFill>
              {clip.kind === 'video' && clip.sourceUrl ? <Video src={clip.sourceUrl} style={commonStyle} /> : null}
              {clip.kind === 'image' && clip.sourceUrl ? <Img src={clip.sourceUrl} style={commonStyle} /> : null}
              {clip.kind === 'text' ? (
                <AbsoluteFill
                  style={{
                    justifyContent: 'center',
                    alignItems: 'center',
                    color: '#e2e8f0',
                    fontSize: 56,
                    fontWeight: 700,
                    textAlign: 'center',
                    padding: '0 96px',
                    letterSpacing: '-0.02em',
                    background: 'linear-gradient(160deg, rgba(59,130,246,0.2), rgba(2,6,23,0.9))',
                    ...style,
                  }}
                >
                  {clip.text || 'Text Clip'}
                </AbsoluteFill>
              ) : null}
              {clip.kind !== 'text' && !clip.sourceUrl ? (
                <AbsoluteFill
                  style={{
                    justifyContent: 'center',
                    alignItems: 'center',
                    color: '#cbd5e1',
                    fontSize: 44,
                    fontWeight: 700,
                    textAlign: 'center',
                    background: 'radial-gradient(circle at 20% 10%, rgba(14,116,144,0.35), rgba(2,6,23,1))',
                  }}
                >
                  Missing Source
                </AbsoluteFill>
              ) : null}
            </AbsoluteFill>
          </Sequence>
        );
      })}
    </AbsoluteFill>
  );
}
