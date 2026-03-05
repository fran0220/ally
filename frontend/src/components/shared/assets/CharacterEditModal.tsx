import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation } from '@tanstack/react-query';

import { apiRequest } from '../../../api/client';
import { shouldShowError } from '../../../lib/error-utils';
import { resolveTaskPresentationState } from '../../../lib/task/presentation';
import { TaskStatusInline } from '../../task/TaskStatusInline';
import { AppIcon } from '../../ui/icons';

export interface CharacterEditModalProps {
  mode: 'asset-hub' | 'project';
  characterId: string;
  characterName: string;
  description: string;
  appearanceIndex?: number;
  changeReason?: string;
  projectId?: string;
  appearanceId?: string;
  descriptionIndex?: number;
  isTaskRunning?: boolean;
  introduction?: string | null;
  onClose: () => void;
  onSave: (characterId: string, appearanceId: string) => void;
  onUpdate?: (newDescription: string) => void;
  onIntroductionUpdate?: (newIntroduction: string) => void;
  onNameUpdate?: (newName: string) => void;
  onRefresh?: () => void;
}

interface AiModifyResult {
  modifiedDescription?: string;
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message;
  return fallback;
}

export function CharacterEditModal({
  mode,
  characterId,
  characterName,
  description,
  appearanceIndex,
  changeReason,
  projectId,
  appearanceId,
  descriptionIndex,
  isTaskRunning = false,
  introduction,
  onClose,
  onSave,
  onUpdate,
  onIntroductionUpdate,
  onNameUpdate,
  onRefresh,
}: CharacterEditModalProps) {
  const { t } = useTranslation('common');

  const appearanceKey = mode === 'asset-hub'
    ? String(appearanceIndex ?? 0)
    : String(appearanceId ?? '');

  const [editingName, setEditingName] = useState(characterName);
  const [editingDescription, setEditingDescription] = useState(description);
  const [editingIntroduction, setEditingIntroduction] = useState(introduction || '');
  const [aiModifyInstruction, setAiModifyInstruction] = useState('');
  const [isAiModifying, setIsAiModifying] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const aiModifyingState = isAiModifying
    ? resolveTaskPresentationState({ phase: 'processing', intent: 'modify', resource: 'image', hasOutput: true })
    : null;
  const savingState = isSaving
    ? resolveTaskPresentationState({ phase: 'processing', intent: 'process', resource: 'text', hasOutput: false })
    : null;
  const taskRunningState = isTaskRunning
    ? resolveTaskPresentationState({ phase: 'processing', intent: 'modify', resource: 'image', hasOutput: true })
    : null;

  const updateNameMutation = useMutation({
    mutationFn: (nextName: string) => {
      if (mode === 'asset-hub') {
        return apiRequest(`/api/asset-hub/characters/${characterId}/name`, {
          method: 'PATCH',
          body: JSON.stringify({ name: nextName }),
        });
      }
      return apiRequest(`/api/novel-promotion/${projectId}/characters/${characterId}/name`, {
        method: 'PATCH',
        body: JSON.stringify({ name: nextName }),
      });
    },
  });

  const persistNameIfNeeded = async () => {
    const nextName = editingName.trim();
    if (!nextName || nextName === characterName) return;
    await updateNameMutation.mutateAsync(nextName);
    onNameUpdate?.(nextName);
  };

  const persistDescription = async () => {
    if (mode === 'asset-hub') {
      await apiRequest(`/api/asset-hub/characters/${characterId}/appearances/${appearanceIndex ?? 0}/description`, {
        method: 'PATCH',
        body: JSON.stringify({ description: editingDescription }),
      });
      return;
    }
    if (!appearanceId) throw new Error('Missing appearanceId');
    await apiRequest(`/api/novel-promotion/${projectId}/characters/${characterId}/appearances/${appearanceId}/description`, {
      method: 'PATCH',
      body: JSON.stringify({ description: editingDescription, descriptionIndex }),
    });
  };

  const persistIntroductionIfNeeded = async () => {
    if (mode !== 'project' || !projectId) return;
    if (editingIntroduction === (introduction || '')) return;
    const nextIntro = editingIntroduction.trim();
    await apiRequest(`/api/novel-promotion/${projectId}/characters/${characterId}/introduction`, {
      method: 'PATCH',
      body: JSON.stringify({ introduction: nextIntro }),
    });
    onIntroductionUpdate?.(nextIntro);
  };

  const handleAiModify = async () => {
    if (!aiModifyInstruction.trim()) return;

    try {
      setIsAiModifying(true);
      const path = mode === 'asset-hub'
        ? `/api/asset-hub/characters/${characterId}/appearances/${appearanceIndex ?? 0}/ai-modify`
        : `/api/novel-promotion/${projectId}/characters/${characterId}/appearances/${appearanceId}/ai-modify`;
      const data = await apiRequest<AiModifyResult>(path, {
        method: 'POST',
        body: JSON.stringify({
          currentDescription: editingDescription,
          modifyInstruction: aiModifyInstruction,
        }),
      });
      if (data?.modifiedDescription) {
        setEditingDescription(data.modifiedDescription);
        onUpdate?.(data.modifiedDescription);
        setAiModifyInstruction('');
      }
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(`${t('assets.modal.modifyFailed')}: ${getErrorMessage(error, t('assets.errors.failed'))}`);
      }
    } finally {
      setIsAiModifying(false);
    }
  };

  const handleSaveName = async () => {
    try {
      await persistNameIfNeeded();
      onRefresh?.();
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(t('assets.modal.saveName') + t('assets.errors.failed'));
      }
    }
  };

  const handleSaveOnly = async () => {
    try {
      setIsSaving(true);
      await persistNameIfNeeded();
      await persistDescription();
      await persistIntroductionIfNeeded();
      onUpdate?.(editingDescription);
      onRefresh?.();
      onClose();
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(getErrorMessage(error, t('assets.errors.saveFailed')));
      }
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveAndGenerate = async () => {
    const savedDescription = editingDescription;
    const savedAppearanceKey = appearanceKey;
    onClose();

    ;(async () => {
      try {
        await persistNameIfNeeded();
        await persistDescription();
        await persistIntroductionIfNeeded();
        onUpdate?.(savedDescription);
        onRefresh?.();
        onSave(characterId, savedAppearanceKey);
      } catch (error: unknown) {
        if (shouldShowError(error)) {
          alert(getErrorMessage(error, t('assets.errors.saveFailed')));
        }
      }
    })();
  };

  return (
    <div className="fixed inset-0 glass-overlay flex items-center justify-center z-50 p-4">
      <div className="glass-surface-modal max-w-2xl w-full max-h-[80vh] flex flex-col">
        <div className="p-6 space-y-4 overflow-y-auto flex-1">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold text-[var(--glass-text-primary)]">
              {t('assets.modal.editCharacter')} - {characterName}
            </h3>
            <button
              onClick={onClose}
              className="glass-btn-base glass-btn-soft w-9 h-9 rounded-full text-[var(--glass-text-tertiary)]"
            >
              <AppIcon name="close" className="w-6 h-6" />
            </button>
          </div>

          <div className="space-y-2">
            <label className="glass-field-label block">
              {t('assets.character.name')}
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                className="glass-input-base flex-1 px-3 py-2"
                placeholder={t('assets.modal.namePlaceholder')}
              />
              {editingName !== characterName && (
                <button
                  onClick={() => { void handleSaveName(); }}
                  disabled={updateNameMutation.isPending || !editingName.trim()}
                  className="glass-btn-base glass-btn-tone-success px-3 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed text-sm whitespace-nowrap"
                >
                  {updateNameMutation.isPending
                    ? t('assets.smartImport.preview.saving')
                    : t('assets.modal.saveName')}
                </button>
              )}
            </div>
          </div>

          {mode === 'project' && (
            <div className="space-y-2">
              <label className="glass-field-label block">
                {t('assets.modal.introduction')}
              </label>
              <textarea
                value={editingIntroduction}
                onChange={(e) => setEditingIntroduction(e.target.value)}
                rows={3}
                className="glass-textarea-base w-full px-3 py-2 resize-none"
                placeholder={t('assets.modal.introductionPlaceholder')}
              />
              <p className="glass-field-hint">
                {t('assets.modal.introductionTip')}
              </p>
            </div>
          )}

          {mode === 'asset-hub' && changeReason && (
            <div className="text-sm text-[var(--glass-text-secondary)]">
              {t('assets.character.appearance')}:
              <span className="ml-1 inline-flex items-center rounded-full px-2 py-0.5 bg-[var(--glass-tone-neutral-bg)] text-[var(--glass-tone-neutral-fg)]">
                {changeReason}
              </span>
            </div>
          )}

          <div className="space-y-2 glass-surface-soft p-4 rounded-[var(--glass-radius-md)] border border-[var(--glass-stroke-base)]">
            <label className="block text-sm font-medium text-[var(--glass-tone-info-fg)] flex items-center gap-2">
              <AppIcon name="bolt" className="w-4 h-4" />
              {t('assets.modal.smartModify')}
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={aiModifyInstruction}
                onChange={(e) => setAiModifyInstruction(e.target.value)}
                placeholder={t('assets.modal.modifyPlaceholderCharacter')}
                className="glass-input-base flex-1 px-3 py-2"
                disabled={isAiModifying}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    void handleAiModify();
                  }
                }}
              />
              <button
                onClick={() => { void handleAiModify(); }}
                disabled={isAiModifying || !aiModifyInstruction.trim()}
                className="glass-btn-base glass-btn-tone-info px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 whitespace-nowrap"
              >
                {isAiModifying ? (
                  <TaskStatusInline state={aiModifyingState} className="text-white [&>span]:text-white [&_svg]:text-white" />
                ) : (
                  <>
                    <AppIcon name="bolt" className="w-4 h-4" />
                    {t('assets.modal.smartModify')}
                  </>
                )}
              </button>
            </div>
            <p className="glass-field-hint">
              {t('assets.modal.aiTipSub')}
            </p>
          </div>

          <div className="space-y-2">
            <label className="glass-field-label block">
              {t('assets.modal.appearancePrompt')}
            </label>
            <textarea
              value={editingDescription}
              onChange={(e) => setEditingDescription(e.target.value)}
              className="glass-textarea-base w-full h-64 px-3 py-2 resize-none"
              placeholder={t('assets.modal.descPlaceholder')}
              disabled={isAiModifying}
            />
          </div>
        </div>

        <div className="flex gap-3 justify-end p-4 border-t border-[var(--glass-stroke-base)] bg-[var(--glass-bg-surface-strong)] rounded-b-lg flex-shrink-0">
          <button
            onClick={onClose}
            className="glass-btn-base glass-btn-secondary px-4 py-2 rounded-[var(--glass-radius-md)]"
            disabled={isSaving}
          >
            {t('assets.common.cancel')}
          </button>
          <button
            onClick={() => { void handleSaveOnly(); }}
            disabled={isSaving || !editingDescription.trim()}
            className="glass-btn-base glass-btn-tone-info px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {isSaving ? (
              <TaskStatusInline state={savingState} className="text-white [&>span]:text-white [&_svg]:text-white" />
            ) : (
              t('assets.modal.saveOnly')
            )}
          </button>
          <button
            onClick={() => { void handleSaveAndGenerate(); }}
            disabled={isSaving || isTaskRunning || !editingDescription.trim()}
            className="glass-btn-base glass-btn-primary px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {isTaskRunning ? (
              <TaskStatusInline state={taskRunningState} className="text-white [&>span]:text-white [&_svg]:text-white" />
            ) : (
              t('assets.modal.saveAndGenerate')
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
