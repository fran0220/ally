import type { DragEvent, RefObject } from 'react';
import { useTranslation } from 'react-i18next';

import { MediaImageWithLoading } from '../../../media/MediaImageWithLoading';
import { AppIcon } from '../../../ui/icons';

interface CharacterCreationPreviewProps {
  referenceImagesBase64: string[];
  fileInputRef: RefObject<HTMLInputElement | null>;
  onDrop: (event: DragEvent<HTMLDivElement>) => void;
  onFileSelect: (files: FileList) => void;
  onClearReference: (index?: number) => void;
}

export default function CharacterCreationPreview({
  referenceImagesBase64,
  fileInputRef,
  onDrop,
  onFileSelect,
  onClearReference,
}: CharacterCreationPreviewProps) {
  const { t } = useTranslation('common');

  return (
    <div
      className="border-2 border-dashed border-[var(--glass-stroke-base)] rounded-[var(--glass-radius-md)] p-4 flex flex-col items-center justify-center cursor-pointer hover:border-[var(--glass-stroke-focus)] hover:bg-[var(--glass-tone-info-bg)] transition-all relative min-h-[120px]"
      onDrop={onDrop}
      onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); }}
      onClick={() => fileInputRef.current?.click()}
    >
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        multiple
        className="hidden"
        onChange={(e) => e.target.files && onFileSelect(e.target.files)}
      />

      {referenceImagesBase64.length > 0 ? (
        <div className="w-full">
          <div className="grid grid-cols-3 gap-2 mb-2">
            {referenceImagesBase64.map((base64, index) => (
              <div key={index} className="relative aspect-square">
                <MediaImageWithLoading
                  src={base64}
                  alt={`${t('referenceImage')} ${index + 1}`}
                  containerClassName="w-full h-full rounded"
                  className="w-full h-full object-cover rounded"
                />
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onClearReference(index);
                  }}
                  className="glass-btn-base glass-btn-tone-danger absolute -top-1 -right-1 w-5 h-5 rounded-full flex items-center justify-center text-xs"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
          <p className="text-xs text-center text-[var(--glass-text-secondary)]">
            {t('assetModal.character.selectedCount', { count: referenceImagesBase64.length })}
          </p>
        </div>
      ) : (
        <>
          <AppIcon name="image" className="w-10 h-10 text-[var(--glass-text-tertiary)] mb-2" />
          <p className="text-sm text-[var(--glass-text-secondary)]">{t('assetModal.character.dropOrClick')}</p>
          <p className="text-xs text-[var(--glass-text-tertiary)] mt-1">{t('assetModal.character.maxReferenceImages')}</p>
        </>
      )}
    </div>
  );
}
