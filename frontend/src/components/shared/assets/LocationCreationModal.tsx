import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { apiRequest } from '../../../api/client';
import { queryKeys } from '../../../lib/query-keys';
import { ART_STYLES } from '../../../lib/constants';
import { shouldShowError } from '../../../lib/error-utils';
import { resolveTaskPresentationState } from '../../../lib/task/presentation';
import { TaskStatusInline } from '../../task/TaskStatusInline';
import { AppIcon } from '../../ui/icons';

export interface LocationCreationModalProps {
  mode: 'asset-hub' | 'project';
  folderId?: string | null;
  projectId?: string;
  onClose: () => void;
  onSuccess: () => void;
}

interface AiDesignResult {
  prompt?: string;
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message;
  return fallback;
}

export function LocationCreationModal({
  mode,
  folderId,
  projectId,
  onClose,
  onSuccess,
}: LocationCreationModalProps) {
  const { t } = useTranslation('common');
  const queryClient = useQueryClient();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [aiInstruction, setAiInstruction] = useState('');
  const [artStyle, setArtStyle] = useState('american-comic');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isAiDesigning, setIsAiDesigning] = useState(false);

  const aiDesigningState = isAiDesigning
    ? resolveTaskPresentationState({ phase: 'processing', intent: 'generate', resource: 'image', hasOutput: false })
    : null;
  const submittingState = isSubmitting
    ? resolveTaskPresentationState({ phase: 'processing', intent: 'generate', resource: 'image', hasOutput: false })
    : null;

  const aiDesignMutation = useMutation({
    mutationFn: (instruction: string) => {
      if (mode === 'asset-hub') {
        return apiRequest<AiDesignResult>('/api/asset-hub/locations/ai-design', {
          method: 'POST',
          body: JSON.stringify(instruction),
        });
      }
      return apiRequest<AiDesignResult>(`/api/novel-promotion/${projectId}/locations/ai-create`, {
        method: 'POST',
        body: JSON.stringify({ userInstruction: instruction }),
      });
    },
  });

  const invalidateQueries = () => {
    if (mode === 'asset-hub') {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub', 'locations'] });
    } else if (projectId) {
      void queryClient.invalidateQueries({ queryKey: queryKeys.projects.assets(projectId) });
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !isSubmitting && !isAiDesigning) {
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [onClose, isSubmitting, isAiDesigning]);

  const handleAiDesign = async () => {
    if (!aiInstruction.trim()) return;

    try {
      setIsAiDesigning(true);
      const data = await aiDesignMutation.mutateAsync(aiInstruction);
      setDescription(data.prompt || '');
      setAiInstruction('');
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(getErrorMessage(error, t('assetModal.errors.aiDesignFailed')));
      }
    } finally {
      setIsAiDesigning(false);
    }
  };

  const handleSubmit = async () => {
    if (!name.trim() || !description.trim()) return;

    try {
      setIsSubmitting(true);

      if (mode === 'asset-hub') {
        await apiRequest('/api/asset-hub/locations', {
          method: 'POST',
          body: JSON.stringify({
            name: name.trim(),
            summary: description.trim(),
            artStyle,
            folderId: folderId ?? null,
          }),
        });
      } else {
        await apiRequest(`/api/novel-promotion/${projectId}/locations`, {
          method: 'POST',
          body: JSON.stringify({
            name: name.trim(),
            description: description.trim(),
            artStyle,
          }),
        });
      }

      invalidateQueries();
      onSuccess();
      onClose();
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(getErrorMessage(error, t('assetModal.errors.createFailed')));
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget && !isSubmitting && !isAiDesigning) {
      onClose();
    }
  };

  return (
    <div
      className="fixed inset-0 glass-overlay flex items-center justify-center z-50 p-4"
      onClick={handleBackdropClick}
    >
      <div className="glass-surface-modal max-w-2xl w-full max-h-[85vh] flex flex-col">
        <div className="p-6 overflow-y-auto flex-1">
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-lg font-semibold text-[var(--glass-text-primary)]">
              {t('assetModal.location.title')}
            </h3>
            <button
              onClick={onClose}
              className="glass-btn-base glass-btn-soft w-8 h-8 rounded-full flex items-center justify-center text-[var(--glass-text-tertiary)]"
            >
              <AppIcon name="close" className="w-5 h-5" />
            </button>
          </div>

          <div className="space-y-5">
            <div className="space-y-2">
              <label className="glass-field-label block">
                {t('assetModal.location.name')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('assetModal.location.namePlaceholder')}
                className="glass-input-base w-full px-3 py-2 text-sm"
              />
            </div>

            <div className="space-y-2">
              <label className="glass-field-label block">
                {t('assetModal.artStyle.title')}
              </label>
              <div className="grid grid-cols-2 gap-2">
                {ART_STYLES.map((style) => (
                  <button
                    key={style.value}
                    type="button"
                    onClick={() => setArtStyle(style.value)}
                    className={`glass-btn-base px-3 py-2 rounded-[var(--glass-radius-md)] text-sm border transition-all justify-start ${artStyle === style.value
                      ? 'glass-btn-tone-info border-[var(--glass-stroke-focus)]'
                      : 'glass-btn-soft border-[var(--glass-stroke-base)] text-[var(--glass-text-secondary)]'
                      }`}
                  >
                    <span>{style.preview}</span>
                    <span>{style.label}</span>
                  </button>
                ))}
              </div>
            </div>

            <div className="glass-surface-soft rounded-[var(--glass-radius-lg)] p-4 space-y-3 border border-[var(--glass-stroke-base)]">
              <div className="flex items-center gap-2 text-sm font-medium text-[var(--glass-tone-info-fg)]">
                <AppIcon name="sparklesAlt" className="w-4 h-4" />
                <span>{t('assetModal.aiDesign.title')} {t('assetModal.common.optional')}</span>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={aiInstruction}
                  onChange={(e) => setAiInstruction(e.target.value)}
                  placeholder={t('assetModal.aiDesign.placeholderLocation')}
                  className="glass-input-base flex-1 px-3 py-2 text-sm"
                  disabled={isAiDesigning}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      void handleAiDesign();
                    }
                  }}
                />
                <button
                  onClick={() => { void handleAiDesign(); }}
                  disabled={isAiDesigning || !aiInstruction.trim()}
                  className="glass-btn-base glass-btn-tone-info px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 text-sm whitespace-nowrap"
                >
                  {isAiDesigning ? (
                    <TaskStatusInline state={aiDesigningState} className="text-white [&>span]:text-white [&_svg]:text-white" />
                  ) : (
                    <>
                      <AppIcon name="sparklesAlt" className="w-4 h-4" />
                      <span>{t('assetModal.aiDesign.generate')}</span>
                    </>
                  )}
                </button>
              </div>
              <p className="glass-field-hint">
                {t('assetModal.aiDesign.tip')}
              </p>
            </div>

            <div className="space-y-2">
              <label className="glass-field-label block">
                {t('assetModal.location.description')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
              </label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder={t('assetModal.location.descPlaceholder')}
                className="glass-textarea-base w-full h-36 px-3 py-2 text-sm resize-none"
                disabled={isAiDesigning}
              />
            </div>
          </div>
        </div>

        <div className="flex gap-3 justify-end p-4 border-t border-[var(--glass-stroke-base)] bg-[var(--glass-bg-surface-strong)] rounded-b-xl flex-shrink-0">
          <button
            onClick={onClose}
            className="glass-btn-base glass-btn-secondary px-4 py-2 rounded-[var(--glass-radius-md)] text-sm"
            disabled={isSubmitting}
          >
            {t('assetModal.common.cancel')}
          </button>
          <button
            onClick={() => { void handleSubmit(); }}
            disabled={isSubmitting || !name.trim() || !description.trim()}
            className="glass-btn-base glass-btn-primary px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed text-sm flex items-center gap-2"
          >
            {isSubmitting ? (
              <TaskStatusInline state={submittingState} className="text-white [&>span]:text-white [&_svg]:text-white" />
            ) : (
              <span>{t('assetModal.common.add')}</span>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
