'use client'

import VoiceCreationModalShell, {
  type VoiceCreationModalShellProps,
} from '@/components/asset-hub/voice-creation/VoiceCreationModalShell'

export type {
  VoiceCreationModalShellProps as VoiceCreationModalProps,
} from '@/components/asset-hub/voice-creation/VoiceCreationModalShell'

export default function VoiceCreationModal(props: VoiceCreationModalShellProps) {
  return <VoiceCreationModalShell {...props} />
}
