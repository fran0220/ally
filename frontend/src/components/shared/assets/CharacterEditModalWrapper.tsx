import {
  CharacterEditModal,
  type CharacterEditModalProps,
} from './CharacterEditModal';

interface CharacterEditModalWrapperProps {
  characterId: string;
  characterName: string;
  appearanceId: number;
  description: string;
  introduction?: string | null;
  descriptionIndex?: number;
  projectId: string;
  onClose: () => void;
  onSave: (characterId: string, appearanceId: number) => void;
  onUpdate: (newDescription: string) => void;
  onIntroductionUpdate?: (newIntroduction: string) => void;
  onNameUpdate?: (newName: string) => void;
  isTaskRunning?: boolean;
}

export default function CharacterEditModalWrapper({
  characterId,
  characterName,
  appearanceId,
  description,
  introduction,
  descriptionIndex,
  projectId,
  onClose,
  onSave,
  onUpdate,
  onIntroductionUpdate,
  onNameUpdate,
  isTaskRunning = false,
}: CharacterEditModalWrapperProps) {
  const handleSave: CharacterEditModalProps['onSave'] = (
    nextCharacterId,
    nextAppearanceId,
  ) => {
    onSave(nextCharacterId, Number(nextAppearanceId));
  };

  return (
    <CharacterEditModal
      mode="project"
      characterId={characterId}
      characterName={characterName}
      appearanceId={String(appearanceId)}
      description={description}
      introduction={introduction}
      descriptionIndex={descriptionIndex}
      projectId={projectId}
      onClose={onClose}
      onSave={handleSave}
      onUpdate={onUpdate}
      onIntroductionUpdate={onIntroductionUpdate}
      onNameUpdate={onNameUpdate}
      isTaskRunning={isTaskRunning}
    />
  );
}
