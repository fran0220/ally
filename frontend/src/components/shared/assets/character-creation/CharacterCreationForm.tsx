import type { DragEvent, RefObject } from 'react';
import { useTranslation } from 'react-i18next';

import { resolveArtStyleOptions } from '../../../../lib/constants';
import { AppIcon } from '../../../ui/icons';
import CharacterCreationPreview from './CharacterCreationPreview';

type Mode = 'asset-hub' | 'project';

interface AvailableCharacter {
  id: string;
  name: string;
  appearances: unknown[];
}

interface CharacterCreationFormProps {
  mode: Mode;
  createMode: 'reference' | 'description';
  setCreateMode: (mode: 'reference' | 'description') => void;
  name: string;
  setName: (value: string) => void;
  description: string;
  setDescription: (value: string) => void;
  aiInstruction: string;
  setAiInstruction: (value: string) => void;
  artStyle: string;
  setArtStyle: (value: string) => void;
  referenceImagesBase64: string[];
  referenceSubMode: 'direct' | 'extract';
  setReferenceSubMode: (mode: 'direct' | 'extract') => void;
  isSubAppearance: boolean;
  setIsSubAppearance: (value: boolean) => void;
  selectedCharacterId: string;
  setSelectedCharacterId: (value: string) => void;
  changeReason: string;
  setChangeReason: (value: string) => void;
  availableCharacters: AvailableCharacter[];
  fileInputRef: RefObject<HTMLInputElement | null>;
  handleDrop: (event: DragEvent<HTMLDivElement>) => void;
  handleFileSelect: (files: FileList) => void;
  handleClearReference: (index?: number) => void;
  handleExtractDescription: () => void;
  handleCreateWithReference: () => void;
  handleAiDesign: () => void;
  handleSubmit: () => void;
  isSubmitting: boolean;
  isAiDesigning: boolean;
  isExtracting: boolean;
}

export default function CharacterCreationForm({
  mode,
  createMode,
  setCreateMode,
  name,
  setName,
  description,
  setDescription,
  aiInstruction,
  setAiInstruction,
  artStyle,
  setArtStyle,
  referenceImagesBase64,
  referenceSubMode,
  setReferenceSubMode,
  isSubAppearance,
  setIsSubAppearance,
  selectedCharacterId,
  setSelectedCharacterId,
  changeReason,
  setChangeReason,
  availableCharacters,
  fileInputRef,
  handleDrop,
  handleFileSelect,
  handleClearReference,
  handleExtractDescription,
  handleCreateWithReference,
  handleAiDesign,
  handleSubmit,
  isSubmitting,
  isAiDesigning,
  isExtracting,
}: CharacterCreationFormProps) {
  const { t } = useTranslation('common');
  const artStyleOptions = resolveArtStyleOptions((key) => t(key));

  return (
    <div className="space-y-5">
      <div className="mb-5">
        {(() => {
          const tabs = ['description', 'reference'] as const;
          const activeIdx = tabs.indexOf(createMode);
          return (
            <div className="rounded-[var(--glass-radius-md)] p-0.5" style={{ background: 'rgba(0,0,0,0.04)' }}>
              <div className="relative grid gap-1" style={{ gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' }}>
                <div
                  className="absolute bottom-0.5 top-0.5 rounded-[var(--glass-radius-sm)] bg-white transition-transform duration-200"
                  style={{
                    boxShadow: '0 1px 4px rgba(0,0,0,0.15), 0 0 0 0.5px rgba(0,0,0,0.06)',
                    width: 'calc(100% / 2)',
                    transform: `translateX(${activeIdx * 100}%)`,
                  }}
                />
                <button
                  onClick={() => setCreateMode('description')}
                  className={`relative z-[1] flex items-center justify-center gap-2 rounded-[var(--glass-radius-sm)] py-2 px-4 text-sm font-medium transition-colors cursor-pointer ${createMode === 'description' ? 'text-[var(--glass-text-primary)]' : 'text-[var(--glass-text-tertiary)] hover:text-[var(--glass-text-secondary)]'}`}
                >
                  <AppIcon name="sparklesAlt" className="w-4 h-4" />
                  <span>{t('assetModal.character.modeDescription')}</span>
                </button>
                <button
                  onClick={() => setCreateMode('reference')}
                  className={`relative z-[1] flex items-center justify-center gap-2 rounded-[var(--glass-radius-sm)] py-2 px-4 text-sm font-medium transition-colors cursor-pointer ${createMode === 'reference' ? 'text-[var(--glass-text-primary)]' : 'text-[var(--glass-text-tertiary)] hover:text-[var(--glass-text-secondary)]'}`}
                >
                  <AppIcon name="image" className="w-4 h-4" />
                  <span>{t('assetModal.character.modeReference')}</span>
                </button>
              </div>
            </div>
          );
        })()}
      </div>

      {mode === 'project' && availableCharacters.length > 0 && (
        <div className="flex items-start gap-3 p-3 glass-surface-soft rounded-[var(--glass-radius-md)] border border-[var(--glass-stroke-base)]">
          <input
            type="checkbox"
            id="isSubAppearance"
            checked={isSubAppearance}
            onChange={(e) => setIsSubAppearance(e.target.checked)}
            className="mt-0.5 w-4 h-4 rounded border-[var(--glass-stroke-base)] text-[var(--glass-tone-info-fg)]"
          />
          <label htmlFor="isSubAppearance" className="flex-1 text-sm cursor-pointer">
            <span className="font-medium text-[var(--glass-text-primary)]">{t('assetModal.character.isSubAppearance')}</span>
            <p className="text-xs text-[var(--glass-text-secondary)] mt-0.5">{t('assetModal.character.isSubAppearanceHint')}</p>
          </label>
        </div>
      )}

      {isSubAppearance && (
        <div className="space-y-2">
          <label className="glass-field-label block">
            {t('assetModal.character.selectMainCharacter')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
          </label>
          <select
            value={selectedCharacterId}
            onChange={(e) => setSelectedCharacterId(e.target.value)}
            className="glass-select-base w-full px-3 py-2 text-sm"
          >
            <option value="">{t('assetModal.character.selectCharacterPlaceholder')}</option>
            {availableCharacters.map((char) => (
              <option key={char.id} value={char.id}>
                {char.name} ({t('assetModal.character.appearancesCount', { count: char.appearances.length })})
              </option>
            ))}
          </select>
        </div>
      )}

      {isSubAppearance && (
        <div className="space-y-2">
          <label className="glass-field-label block">
            {t('assetModal.character.changeReason')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
          </label>
          <input
            type="text"
            value={changeReason}
            onChange={(e) => setChangeReason(e.target.value)}
            placeholder={t('assetModal.character.changeReasonPlaceholder')}
            className="glass-input-base w-full px-3 py-2 text-sm"
          />
        </div>
      )}

      {!isSubAppearance && (
        <div className="space-y-2">
          <label className="glass-field-label block">
            {t('assetModal.character.name')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('assetModal.character.namePlaceholder')}
            className="glass-input-base w-full px-3 py-2 text-sm"
          />
        </div>
      )}

      {!isSubAppearance && (
        <div className="space-y-2">
          <label className="glass-field-label block">
            {t('assetModal.artStyle.title')}
          </label>
          <div className="grid grid-cols-2 gap-2">
            {artStyleOptions.map((style) => (
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
      )}

      {createMode === 'reference' && (
        <div className="glass-surface-soft rounded-[var(--glass-radius-lg)] p-4 space-y-3 border border-[var(--glass-stroke-base)]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-medium text-[var(--glass-tone-info-fg)]">
              <AppIcon name="image" className="w-4 h-4" />
              <span>{t('assetModal.character.uploadReference')}</span>
            </div>
            <span className="text-xs text-[var(--glass-text-tertiary)]">{t('assetModal.character.pasteHint')}</span>
          </div>

          <div className="glass-surface flex items-center gap-2 p-2 rounded-[var(--glass-radius-md)]">
            <span className="text-xs text-[var(--glass-text-secondary)] shrink-0">{t('assetModal.character.generationMode')}：</span>
            {(() => {
              const subTabs = ['direct', 'extract'] as const;
              const subIdx = subTabs.indexOf(referenceSubMode);
              return (
                <div className="flex-1 rounded-[var(--glass-radius-sm)] p-0.5" style={{ background: 'rgba(0,0,0,0.04)' }}>
                  <div className="relative grid gap-1" style={{ gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' }}>
                    <div
                      className="absolute bottom-0.5 top-0.5 rounded-sm bg-white transition-transform duration-200"
                      style={{
                        boxShadow: '0 1px 4px rgba(0,0,0,0.15), 0 0 0 0.5px rgba(0,0,0,0.06)',
                        width: 'calc(100% / 2)',
                        transform: `translateX(${subIdx * 100}%)`,
                      }}
                    />
                    <button
                      onClick={() => setReferenceSubMode('direct')}
                      className={`relative z-[1] px-3 py-1.5 text-xs rounded-sm transition-colors cursor-pointer ${referenceSubMode === 'direct' ? 'text-[var(--glass-text-primary)] font-medium' : 'text-[var(--glass-text-tertiary)] hover:text-[var(--glass-text-secondary)]'}`}
                    >
                      {t('assetModal.character.directGenerate')}
                    </button>
                    <button
                      onClick={() => setReferenceSubMode('extract')}
                      className={`relative z-[1] px-3 py-1.5 text-xs rounded-sm transition-colors cursor-pointer ${referenceSubMode === 'extract' ? 'text-[var(--glass-text-primary)] font-medium' : 'text-[var(--glass-text-tertiary)] hover:text-[var(--glass-text-secondary)]'}`}
                    >
                      {t('assetModal.character.extractPrompt')}
                    </button>
                  </div>
                </div>
              );
            })()}
          </div>

          {referenceSubMode === 'extract' && (
            <button
              onClick={handleExtractDescription}
              disabled={isExtracting || referenceImagesBase64.length === 0}
              className="glass-btn-base glass-btn-tone-info w-full px-3 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              {isExtracting ? t('assetModal.aiDesign.generating') : t('assetModal.character.extractFirst')}
            </button>
          )}

          <CharacterCreationPreview
            referenceImagesBase64={referenceImagesBase64}
            fileInputRef={fileInputRef}
            onDrop={handleDrop}
            onFileSelect={handleFileSelect}
            onClearReference={handleClearReference}
          />

          <button
            onClick={handleCreateWithReference}
            disabled={isSubmitting || !name.trim() || referenceImagesBase64.length === 0}
            className={`glass-btn-base w-full px-4 py-2.5 rounded-[var(--glass-radius-md)] transition-all disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center gap-2 text-sm ${referenceSubMode === 'extract'
              ? 'glass-btn-tone-info'
              : 'glass-btn-primary'
              }`}
          >
            {isSubmitting ? t('assetModal.common.creating') : t('assetModal.character.convertToSheet')}
          </button>
        </div>
      )}

      {createMode === 'description' && (
        <>
          {!isSubAppearance && (
            <div className="glass-surface-soft rounded-[var(--glass-radius-lg)] p-4 space-y-3 border border-[var(--glass-stroke-base)]">
              <div className="flex items-center gap-2 text-sm font-medium text-[var(--glass-tone-info-fg)]">
                <AppIcon name="sparklesAlt" className="w-4 h-4" />
                <span>{t('assetModal.aiDesign.title')}</span>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={aiInstruction}
                  onChange={(e) => setAiInstruction(e.target.value)}
                  placeholder={t('assetModal.aiDesign.placeholder')}
                  className="glass-input-base flex-1 px-3 py-2 text-sm"
                  disabled={isAiDesigning}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      handleAiDesign();
                    }
                  }}
                />
                <button
                  onClick={handleAiDesign}
                  disabled={isAiDesigning || !aiInstruction.trim()}
                  className="glass-btn-base glass-btn-tone-info px-4 py-2 rounded-[var(--glass-radius-md)] disabled:opacity-50 disabled:cursor-not-allowed text-sm whitespace-nowrap"
                >
                  {isAiDesigning ? t('assetModal.aiDesign.generating') : t('assetModal.aiDesign.generate')}
                </button>
              </div>
            </div>
          )}

          <div className="space-y-2">
            <label className="glass-field-label block">
              {isSubAppearance ? t('assetModal.character.modifyDescription') : t('assetModal.character.description')} <span className="text-[var(--glass-tone-danger-fg)]">*</span>
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={4}
              placeholder={isSubAppearance
                ? t('assetModal.character.modifyDescriptionPlaceholder')
                : t('assetModal.character.descPlaceholder')}
              className="glass-textarea-base w-full px-3 py-2 text-sm resize-none"
            />
          </div>

          <button
            onClick={handleSubmit}
            disabled={isSubmitting || (isSubAppearance
              ? !selectedCharacterId.trim() || !changeReason.trim() || !description.trim()
              : !name.trim() || !description.trim())}
            className="glass-btn-base glass-btn-primary w-full px-4 py-2.5 rounded-[var(--glass-radius-md)] disabled:opacity-40 disabled:cursor-not-allowed text-sm"
          >
            {isSubmitting ? t('assetModal.common.adding') : t('assetModal.common.add')}
          </button>
        </>
      )}
    </div>
  );
}
