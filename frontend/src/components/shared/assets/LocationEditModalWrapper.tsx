import { LocationEditModal } from './LocationEditModal';

interface LocationEditModalWrapperProps {
  locationId: string;
  locationName: string;
  description: string;
  projectId: string;
  onClose: () => void;
  onSave: (locationId: string) => void;
  onUpdate: (newDescription: string) => void;
  onNameUpdate?: (newName: string) => void;
  isTaskRunning?: boolean;
}

export default function LocationEditModalWrapper({
  locationId,
  locationName,
  description,
  projectId,
  onClose,
  onSave,
  onUpdate,
  onNameUpdate,
  isTaskRunning = false,
}: LocationEditModalWrapperProps) {
  return (
    <LocationEditModal
      mode="project"
      locationId={locationId}
      locationName={locationName}
      description={description}
      projectId={projectId}
      onClose={onClose}
      onSave={onSave}
      onUpdate={onUpdate}
      onNameUpdate={onNameUpdate}
      isTaskRunning={isTaskRunning}
    />
  );
}
