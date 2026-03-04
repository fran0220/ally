import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';

import { MediaImageWithLoading } from '../media';
import { AppIcon } from './icons';
import { resolveOriginalImageUrl, toDisplayImageUrl } from '../../lib/media/image-url';

interface ImagePreviewModalProps {
  imageUrl: string | null;
  onClose: () => void;
}

export function ImagePreviewModal({ imageUrl, onClose }: ImagePreviewModalProps) {
  const { t } = useTranslation('common');

  useEffect(() => {
    document.body.style.overflow = 'hidden';
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => {
      document.body.style.overflow = 'unset';
      document.removeEventListener('keydown', handleEscape);
    };
  }, [onClose]);

  if (!imageUrl) {
    return null;
  }

  const displayImageUrl = toDisplayImageUrl(imageUrl);
  const originalImageUrl = resolveOriginalImageUrl(imageUrl) ?? displayImageUrl;
  if (!displayImageUrl) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-[9999] flex items-center justify-center bg-[var(--glass-overlay)] backdrop-blur-sm"
      onClick={onClose}
      style={{ margin: 0, padding: 0 }}
    >
      <div className="relative max-h-[90vh] max-w-7xl p-4">
        <button
          type="button"
          onClick={onClose}
          className="absolute right-6 top-6 z-10 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--glass-overlay)] text-white transition-colors hover:bg-[var(--glass-overlay-strong)]"
        >
          <AppIcon name="close" className="h-6 w-6" />
        </button>

        {originalImageUrl ? (
          <a
            href={originalImageUrl}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(event) => event.stopPropagation()}
            className="absolute right-20 top-6 z-10 inline-flex h-10 items-center rounded-full bg-[var(--glass-overlay)] px-3 text-sm text-white transition-colors hover:bg-[var(--glass-overlay-strong)]"
          >
            {t('viewOriginal')}
          </a>
        ) : null}

        <MediaImageWithLoading
          src={displayImageUrl}
          alt={t('preview')}
          containerClassName="max-h-[90vh] max-w-full"
          className="max-h-[90vh] max-w-full rounded-lg object-contain shadow-2xl"
          onClick={(event) => event.stopPropagation()}
        />
      </div>
    </div>
  );
}

export default ImagePreviewModal;
