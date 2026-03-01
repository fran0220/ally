import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { apiRequest } from '../../../../../api/client';
import { queryKeys } from '../../../../../lib/query-keys';
import { shouldShowError } from '../../../../../lib/error-utils';

type Mode = 'asset-hub' | 'project';

interface UseCharacterCreationSubmitParams {
  mode: Mode;
  folderId?: string | null;
  projectId?: string;
  name: string;
  description: string;
  aiInstruction: string;
  artStyle: string;
  referenceImagesBase64: string[];
  referenceSubMode: 'direct' | 'extract';
  isSubAppearance: boolean;
  selectedCharacterId: string;
  changeReason: string;
  setDescription: (value: string) => void;
  setAiInstruction: (value: string) => void;
  onSuccess: () => void;
  onClose: () => void;
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message;
  return fallback;
}

interface AiDesignResult {
  prompt?: string;
}

interface ExtractResult {
  description?: string;
}

interface UploadResult {
  url?: string;
}

export function useCharacterCreationSubmit({
  mode,
  folderId,
  projectId,
  name,
  description,
  aiInstruction,
  artStyle,
  referenceImagesBase64,
  referenceSubMode,
  isSubAppearance,
  selectedCharacterId,
  changeReason,
  setDescription,
  setAiInstruction,
  onSuccess,
  onClose,
}: UseCharacterCreationSubmitParams) {
  const { t } = useTranslation('common');
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isAiDesigning, setIsAiDesigning] = useState(false);
  const [isExtracting, setIsExtracting] = useState(false);

  const uploadTempMutation = useMutation({
    mutationFn: (imageBase64: string) => {
      const path = mode === 'asset-hub'
        ? '/api/asset-hub/temp-media'
        : `/api/novel-promotion/${projectId}/temp-media`;
      return apiRequest<UploadResult>(path, {
        method: 'POST',
        body: JSON.stringify({ imageBase64 }),
      });
    },
  });

  const uploadReferenceImages = useCallback(async () => {
    return Promise.all(
      referenceImagesBase64.map(async (base64) => {
        const data = await uploadTempMutation.mutateAsync(base64);
        if (!data.url) throw new Error(t('assetModal.errors.uploadFailed'));
        return data.url;
      }),
    );
  }, [referenceImagesBase64, t, uploadTempMutation]);

  const invalidateQueries = useCallback(() => {
    if (mode === 'asset-hub') {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub', 'characters'] });
    } else if (projectId) {
      void queryClient.invalidateQueries({ queryKey: queryKeys.projects.assets(projectId) });
    }
  }, [mode, projectId, queryClient]);

  const handleExtractDescription = useCallback(async () => {
    if (referenceImagesBase64.length === 0) return;

    try {
      setIsExtracting(true);
      const referenceImageUrls = await uploadReferenceImages();
      const path = mode === 'asset-hub'
        ? '/api/asset-hub/characters/extract-description'
        : `/api/novel-promotion/${projectId}/characters/extract-description`;
      const result = await apiRequest<ExtractResult>(path, {
        method: 'POST',
        body: JSON.stringify({ referenceImageUrls }),
      });
      if (result?.description) {
        setDescription(result.description);
      }
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(getErrorMessage(error, t('assetModal.errors.extractDescriptionFailed')));
      }
    } finally {
      setIsExtracting(false);
    }
  }, [mode, projectId, referenceImagesBase64.length, setDescription, t, uploadReferenceImages]);

  const handleCreateWithReference = useCallback(async () => {
    if (!name.trim() || referenceImagesBase64.length === 0) return;

    try {
      setIsSubmitting(true);
      const referenceImageUrls = await uploadReferenceImages();

      let finalDescription = description.trim();
      if (referenceSubMode === 'extract') {
        const extractPath = mode === 'asset-hub'
          ? '/api/asset-hub/characters/extract-description'
          : `/api/novel-promotion/${projectId}/characters/extract-description`;
        const result = await apiRequest<ExtractResult>(extractPath, {
          method: 'POST',
          body: JSON.stringify({ referenceImageUrls }),
        });
        finalDescription = result?.description || finalDescription;
      }

      const createPath = mode === 'asset-hub'
        ? '/api/asset-hub/characters'
        : `/api/novel-promotion/${projectId}/characters`;
      const body: Record<string, unknown> = {
        name: name.trim(),
        description: finalDescription || t('assetModal.character.defaultDescription', { name: name.trim() }),
        artStyle,
        generateFromReference: true,
        referenceImageUrls,
      };
      if (referenceSubMode === 'extract') {
        body.customDescription = finalDescription;
      }
      if (mode === 'asset-hub') {
        body.folderId = folderId ?? null;
      }

      await apiRequest(createPath, {
        method: 'POST',
        body: JSON.stringify(body),
      });

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
  }, [
    artStyle, description, folderId, invalidateQueries, mode, name, onClose,
    onSuccess, projectId, referenceImagesBase64.length, referenceSubMode, t,
    uploadReferenceImages,
  ]);

  const handleAiDesign = useCallback(async () => {
    if (!aiInstruction.trim()) return;

    try {
      setIsAiDesigning(true);
      const path = mode === 'asset-hub'
        ? '/api/asset-hub/characters/ai-design'
        : `/api/novel-promotion/${projectId}/characters/ai-create`;
      const body = mode === 'asset-hub'
        ? aiInstruction
        : { userInstruction: aiInstruction };
      const result = await apiRequest<AiDesignResult>(path, {
        method: 'POST',
        body: JSON.stringify(body),
      });

      if (result?.prompt) {
        setDescription(result.prompt);
        setAiInstruction('');
      }
    } catch (error: unknown) {
      if (shouldShowError(error)) {
        alert(getErrorMessage(error, t('assetModal.errors.aiDesignFailed')));
      }
    } finally {
      setIsAiDesigning(false);
    }
  }, [aiInstruction, mode, projectId, setAiInstruction, setDescription, t]);

  const handleSubmit = useCallback(async () => {
    if (isSubAppearance) {
      if (!selectedCharacterId.trim() || !changeReason.trim() || !description.trim()) return;
      try {
        setIsSubmitting(true);
        await apiRequest(`/api/novel-promotion/${projectId}/characters/${selectedCharacterId}/appearances`, {
          method: 'POST',
          body: JSON.stringify({
            characterId: selectedCharacterId,
            changeReason: changeReason.trim(),
            description: description.trim(),
          }),
        });
        invalidateQueries();
        onSuccess();
        onClose();
      } catch (error: unknown) {
        if (shouldShowError(error)) {
          alert(getErrorMessage(error, t('assetModal.errors.addSubAppearanceFailed')));
        }
      } finally {
        setIsSubmitting(false);
      }
      return;
    }

    if (!name.trim() || !description.trim()) return;
    try {
      setIsSubmitting(true);
      const createPath = mode === 'asset-hub'
        ? '/api/asset-hub/characters'
        : `/api/novel-promotion/${projectId}/characters`;
      const body: Record<string, unknown> = {
        name: name.trim(),
        description: description.trim(),
        artStyle,
      };
      if (mode === 'asset-hub') {
        body.folderId = folderId ?? null;
      }
      await apiRequest(createPath, {
        method: 'POST',
        body: JSON.stringify(body),
      });
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
  }, [
    artStyle, changeReason, description, folderId, invalidateQueries,
    isSubAppearance, mode, name, onClose, onSuccess, projectId,
    selectedCharacterId, t,
  ]);

  return {
    isSubmitting,
    isAiDesigning,
    isExtracting,
    handleExtractDescription,
    handleCreateWithReference,
    handleAiDesign,
    handleSubmit,
  };
}
