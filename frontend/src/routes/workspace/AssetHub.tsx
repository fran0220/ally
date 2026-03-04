import { type FormEvent, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

import { apiRequest } from '../../api/client';
import {
  type AssetCharacter,
  type AssetLocation,
  bindCharacterVoice,
  createAssetCharacter,
  createAssetFolder,
  createAssetLocation,
  createAssetVoice,
  deleteAssetCharacter,
  deleteAssetFolder,
  deleteAssetLocation,
  deleteAssetVoice,
  listAssetCharacters,
  listAssetFolders,
  listAssetLocations,
  listAssetVoices,
  updateAssetCharacter,
  updateAssetCharacterAppearance,
  updateAssetFolder,
  updateAssetLocation,
} from '../../api/asset-hub';
import { MediaImageWithLoading } from '../../components/media';
import { ImagePreviewModal } from '../../components/ui/ImagePreviewModal';
import {
  GlassButton,
  GlassChip,
  GlassField,
  GlassInput,
  GlassModalShell,
  GlassSurface,
  GlassTextarea,
} from '../../components/ui/primitives';
import { useTaskSse } from '../../hooks/use-task-sse';
import { queryKeys } from '../../lib/query-keys';

interface FolderFormState {
  name: string;
}

interface AssetFormState {
  name: string;
  description: string;
}

interface CharacterEditState {
  characterId: string;
  appearanceIndex: number;
  name: string;
  description: string;
  originalName: string;
  originalDescription: string;
}

interface LocationEditState {
  locationId: string;
  name: string;
  summary: string;
  originalName: string;
  originalSummary: string;
}

interface VoiceBindState {
  characterId: string;
  characterName: string;
  selectedVoiceId: string;
}

const EMPTY_FOLDER: FolderFormState = { name: '' };
const EMPTY_ASSET: AssetFormState = { name: '', description: '' };

function getCharacterPreview(character: AssetCharacter): string | null {
  const appearance = character.appearances[0];
  if (!appearance) {
    return null;
  }
  if (appearance.selectedIndex !== null && appearance.selectedIndex >= 0) {
    return appearance.imageUrls[appearance.selectedIndex] ?? appearance.imageUrl ?? null;
  }
  return appearance.imageUrl ?? appearance.imageUrls[0] ?? null;
}

function getLocationPreview(location: AssetLocation): string | null {
  const selected = location.images.find((image) => image.isSelected);
  return selected?.imageUrl ?? location.images[0]?.imageUrl ?? null;
}

export function AssetHub() {
  const { t } = useTranslation(['assetHub', 'common']);
  const queryClient = useQueryClient();

  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null);
  const [folderForm, setFolderForm] = useState<FolderFormState>(EMPTY_FOLDER);
  const [characterForm, setCharacterForm] = useState<AssetFormState>(EMPTY_ASSET);
  const [locationForm, setLocationForm] = useState<AssetFormState>(EMPTY_ASSET);
  const [voiceForm, setVoiceForm] = useState<AssetFormState>(EMPTY_ASSET);

  const [folderModalOpen, setFolderModalOpen] = useState(false);
  const [editingFolderId, setEditingFolderId] = useState<string | null>(null);
  const [characterModalOpen, setCharacterModalOpen] = useState(false);
  const [locationModalOpen, setLocationModalOpen] = useState(false);
  const [voiceModalOpen, setVoiceModalOpen] = useState(false);

  const [previewImageUrl, setPreviewImageUrl] = useState<string | null>(null);
  const [characterEdit, setCharacterEdit] = useState<CharacterEditState | null>(null);
  const [locationEdit, setLocationEdit] = useState<LocationEditState | null>(null);
  const [voiceBind, setVoiceBind] = useState<VoiceBindState | null>(null);
  const [isCharacterSaving, setIsCharacterSaving] = useState(false);
  const [isLocationSaving, setIsLocationSaving] = useState(false);

  const foldersQuery = useQuery({
    queryKey: queryKeys.assetHub.folders(),
    queryFn: listAssetFolders,
  });
  const charactersQuery = useQuery({
    queryKey: queryKeys.assetHub.characters(selectedFolderId),
    queryFn: () => listAssetCharacters(selectedFolderId),
  });
  const locationsQuery = useQuery({
    queryKey: queryKeys.assetHub.locations(selectedFolderId),
    queryFn: () => listAssetLocations(selectedFolderId),
  });
  const voicesQuery = useQuery({
    queryKey: queryKeys.assetHub.voices(selectedFolderId),
    queryFn: () => listAssetVoices(selectedFolderId),
  });
  const allVoicesQuery = useQuery({
    queryKey: queryKeys.assetHub.voices(null),
    queryFn: () => listAssetVoices(null),
  });

  const folderCreateMutation = useMutation({
    mutationFn: (name: string) => createAssetFolder(name),
    onSuccess: () => {
      setFolderModalOpen(false);
      setEditingFolderId(null);
      setFolderForm(EMPTY_FOLDER);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const folderUpdateMutation = useMutation({
    mutationFn: ({ folderId, name }: { folderId: string; name: string }) => updateAssetFolder(folderId, name),
    onSuccess: () => {
      setFolderModalOpen(false);
      setEditingFolderId(null);
      setFolderForm(EMPTY_FOLDER);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const folderDeleteMutation = useMutation({
    mutationFn: (folderId: string) => deleteAssetFolder(folderId),
    onSuccess: (_, folderId) => {
      if (selectedFolderId === folderId) {
        setSelectedFolderId(null);
      }
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const characterMutation = useMutation({
    mutationFn: () =>
      createAssetCharacter({
        name: characterForm.name.trim(),
        folderId: selectedFolderId,
        profileData: characterForm.description.trim()
          ? { description: characterForm.description.trim() }
          : undefined,
      }),
    onSuccess: () => {
      setCharacterModalOpen(false);
      setCharacterForm(EMPTY_ASSET);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const locationMutation = useMutation({
    mutationFn: () =>
      createAssetLocation({
        name: locationForm.name.trim(),
        summary: locationForm.description.trim(),
        folderId: selectedFolderId,
        description: locationForm.description.trim(),
      }),
    onSuccess: () => {
      setLocationModalOpen(false);
      setLocationForm(EMPTY_ASSET);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const voiceCreateMutation = useMutation({
    mutationFn: () =>
      createAssetVoice({
        name: voiceForm.name.trim(),
        description: voiceForm.description.trim() || undefined,
        folderId: selectedFolderId,
      }),
    onSuccess: () => {
      setVoiceModalOpen(false);
      setVoiceForm(EMPTY_ASSET);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const characterDeleteMutation = useMutation({
    mutationFn: (characterId: string) => deleteAssetCharacter(characterId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const locationDeleteMutation = useMutation({
    mutationFn: (locationId: string) => deleteAssetLocation(locationId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const voiceDeleteMutation = useMutation({
    mutationFn: (voiceId: string) => deleteAssetVoice(voiceId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const voiceBindMutation = useMutation({
    mutationFn: async ({ characterId, voiceId }: { characterId: string; voiceId: string }) => {
      const voice = (allVoicesQuery.data?.voices ?? []).find((item) => item.id === voiceId);
      await bindCharacterVoice(characterId, {
        globalVoiceId: voiceId || null,
        customVoiceUrl: voice?.customVoiceUrl ?? null,
        voiceType: voice?.voiceType ?? null,
      });
    },
    onSuccess: () => {
      setVoiceBind(null);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const regenerateMutation = useMutation({
    mutationFn: (payload: { type: 'character' | 'location'; id: string; appearanceIndex?: number }) =>
      apiRequest<{ success: boolean; taskId?: string; async?: boolean }>('/api/asset-hub/generate-image', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });

  const { connected, events } = useTaskSse({ projectId: 'global-asset-hub', enabled: true });

  const folders = foldersQuery.data?.folders ?? [];
  const characters = charactersQuery.data?.characters ?? [];
  const locations = locationsQuery.data?.locations ?? [];
  const voices = voicesQuery.data?.voices ?? [];
  const allVoices = allVoicesQuery.data?.voices ?? [];

  const activeFolderName = useMemo(() => {
    if (!selectedFolderId) {
      return t('allAssets');
    }
    return folders.find((folder) => folder.id === selectedFolderId)?.name ?? t('allAssets');
  }, [folders, selectedFolderId, t]);

  function openCreateFolderModal() {
    setEditingFolderId(null);
    setFolderForm(EMPTY_FOLDER);
    setFolderModalOpen(true);
  }

  function openEditFolderModal(folder: { id: string; name: string }) {
    setEditingFolderId(folder.id);
    setFolderForm({ name: folder.name });
    setFolderModalOpen(true);
  }

  async function onCreateFolder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!folderForm.name.trim()) {
      return;
    }
    if (editingFolderId) {
      await folderUpdateMutation.mutateAsync({ folderId: editingFolderId, name: folderForm.name.trim() });
      return;
    }
    await folderCreateMutation.mutateAsync(folderForm.name.trim());
  }

  async function onCreateCharacter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!characterForm.name.trim()) {
      return;
    }
    await characterMutation.mutateAsync();
  }

  async function onCreateLocation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!locationForm.name.trim()) {
      return;
    }
    await locationMutation.mutateAsync();
  }

  async function onCreateVoice(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!voiceForm.name.trim()) {
      return;
    }
    await voiceCreateMutation.mutateAsync();
  }

  async function submitCharacterEdit(regenerate: boolean) {
    if (!characterEdit) {
      return;
    }
    setIsCharacterSaving(true);
    try {
      const nextName = characterEdit.name.trim();
      const nextDescription = characterEdit.description.trim();
      if (!nextName || !nextDescription) {
        return;
      }

      if (nextName !== characterEdit.originalName) {
        await updateAssetCharacter(characterEdit.characterId, { name: nextName });
      }
      if (nextDescription !== characterEdit.originalDescription) {
        await updateAssetCharacterAppearance(characterEdit.characterId, characterEdit.appearanceIndex, {
          description: nextDescription,
        });
      }
      if (regenerate) {
        await regenerateMutation.mutateAsync({
          type: 'character',
          id: characterEdit.characterId,
          appearanceIndex: characterEdit.appearanceIndex,
        });
      }
      setCharacterEdit(null);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    } finally {
      setIsCharacterSaving(false);
    }
  }

  async function submitLocationEdit(regenerate: boolean) {
    if (!locationEdit) {
      return;
    }
    setIsLocationSaving(true);
    try {
      const nextName = locationEdit.name.trim();
      const nextSummary = locationEdit.summary.trim();
      if (!nextName || !nextSummary) {
        return;
      }

      if (nextName !== locationEdit.originalName || nextSummary !== locationEdit.originalSummary) {
        await updateAssetLocation(locationEdit.locationId, {
          name: nextName,
          summary: nextSummary,
        });
      }
      if (regenerate) {
        await regenerateMutation.mutateAsync({
          type: 'location',
          id: locationEdit.locationId,
        });
      }
      setLocationEdit(null);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    } finally {
      setIsLocationSaving(false);
    }
  }

  return (
    <main className="page-shell py-8 md:py-10">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="glass-page-title">{t('title')}</h1>
          <p className="glass-page-subtitle">{t('description')}</p>
          <p className="mt-2 text-xs text-[var(--glass-text-tertiary)]">
            {t('modelHint')}{' '}
            <Link className="text-[var(--glass-tone-info-fg)] hover:underline" to="/profile">
              {t('modelHintLink')}
            </Link>
            {t('modelHintSuffix')}
          </p>
        </div>
        <div className="flex items-center gap-2 text-xs">
          <GlassChip tone={connected ? 'success' : 'warning'}>{connected ? 'SSE Connected' : 'SSE Reconnecting'}</GlassChip>
          <GlassChip tone="neutral">{events.length} events</GlassChip>
        </div>
      </header>

      <div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
        <GlassSurface>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-[var(--glass-text-secondary)]">{t('folders')}</h2>
            <GlassButton size="sm" variant="soft" onClick={openCreateFolderModal}>
              +
            </GlassButton>
          </div>
          <div className="space-y-2">
            <button
              type="button"
              className={`glass-list-row w-full text-left ${selectedFolderId === null ? 'border-[var(--glass-stroke-focus)]' : ''}`}
              onClick={() => setSelectedFolderId(null)}
            >
              <span>{t('allAssets')}</span>
            </button>
            {folders.map((folder) => (
              <div key={folder.id} className={`glass-list-row ${selectedFolderId === folder.id ? 'border-[var(--glass-stroke-focus)]' : ''}`}>
                <button type="button" className="flex-1 text-left" onClick={() => setSelectedFolderId(folder.id)}>
                  {folder.name}
                </button>
                <div className="flex items-center gap-1">
                  <GlassButton type="button" variant="ghost" size="sm" onClick={() => openEditFolderModal(folder)}>
                    {t('common:edit')}
                  </GlassButton>
                  <GlassButton
                    type="button"
                    variant="danger"
                    size="sm"
                    loading={folderDeleteMutation.isPending}
                    onClick={() => {
                      if (window.confirm(t('confirmDeleteFolder'))) {
                        folderDeleteMutation.mutate(folder.id);
                      }
                    }}
                  >
                    {t('common:delete')}
                  </GlassButton>
                </div>
              </div>
            ))}
            {!foldersQuery.isLoading && folders.length === 0 ? (
              <p className="pt-2 text-xs text-[var(--glass-text-tertiary)]">{t('noFolders')}</p>
            ) : null}
          </div>
        </GlassSurface>

        <div className="space-y-4">
          <GlassSurface>
            <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
              <div>
                <h2 className="text-lg font-semibold">{activeFolderName}</h2>
                <p className="text-sm text-[var(--glass-text-secondary)]">{t('emptyStateHint')}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <GlassButton size="sm" variant="soft" onClick={() => setCharacterModalOpen(true)}>
                  {t('addCharacter')}
                </GlassButton>
                <GlassButton size="sm" variant="soft" onClick={() => setLocationModalOpen(true)}>
                  {t('addLocation')}
                </GlassButton>
                <GlassButton size="sm" variant="soft" onClick={() => setVoiceModalOpen(true)}>
                  {t('addVoice')}
                </GlassButton>
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-3">
              <article className="glass-kpi p-4">
                <p className="text-xs uppercase tracking-wide text-[var(--glass-text-tertiary)]">{t('characters')}</p>
                <p className="mt-2 text-2xl font-semibold">{characters.length}</p>
              </article>
              <article className="glass-kpi p-4">
                <p className="text-xs uppercase tracking-wide text-[var(--glass-text-tertiary)]">{t('locations')}</p>
                <p className="mt-2 text-2xl font-semibold">{locations.length}</p>
              </article>
              <article className="glass-kpi p-4">
                <p className="text-xs uppercase tracking-wide text-[var(--glass-text-tertiary)]">{t('voices')}</p>
                <p className="mt-2 text-2xl font-semibold">{voices.length}</p>
              </article>
            </div>
          </GlassSurface>

          <GlassSurface>
            <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">{t('characters')}</h3>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {characters.map((character) => {
                const appearance = character.appearances[0];
                const preview = getCharacterPreview(character);
                return (
                  <article key={character.id} className="glass-list-row flex-col items-stretch gap-2 p-2">
                    <button
                      type="button"
                      className="block w-full text-left"
                      onClick={() => {
                        if (preview) {
                          setPreviewImageUrl(preview);
                        }
                      }}
                    >
                      <div className="mb-2 overflow-hidden rounded-lg bg-[var(--glass-bg-muted)]">
                        {preview ? (
                          <MediaImageWithLoading
                            src={preview}
                            alt={character.name}
                            containerClassName="h-28 w-full"
                            className="h-28 w-full object-cover"
                          />
                        ) : (
                          <div className="flex h-28 items-center justify-center text-xs text-[var(--glass-text-tertiary)]">
                            {t('emptyState')}
                          </div>
                        )}
                      </div>
                      <p className="text-sm font-semibold">{character.name}</p>
                      <p className="text-xs text-[var(--glass-text-tertiary)]">
                        {character.appearances.length} appearances
                      </p>
                    </button>
                    <div className="flex flex-wrap gap-1">
                      <GlassButton
                        size="sm"
                        variant="ghost"
                        onClick={() => {
                          setCharacterEdit({
                            characterId: character.id,
                            appearanceIndex: appearance?.appearanceIndex ?? 0,
                            name: character.name,
                            description: appearance?.description ?? '',
                            originalName: character.name,
                            originalDescription: appearance?.description ?? '',
                          });
                        }}
                      >
                        {t('common:edit')}
                      </GlassButton>
                      <GlassButton
                        size="sm"
                        variant="soft"
                        onClick={() => {
                          setVoiceBind({
                            characterId: character.id,
                            characterName: character.name,
                            selectedVoiceId: character.globalVoiceId ?? '',
                          });
                        }}
                      >
                        {t('voicePickerTitle')}
                      </GlassButton>
                      <GlassButton
                        size="sm"
                        variant="soft"
                        loading={regenerateMutation.isPending}
                        onClick={() => {
                          regenerateMutation.mutate({
                            type: 'character',
                            id: character.id,
                            appearanceIndex: appearance?.appearanceIndex ?? 0,
                          });
                        }}
                      >
                        {t('regenerate')}
                      </GlassButton>
                      <GlassButton
                        size="sm"
                        variant="danger"
                        loading={characterDeleteMutation.isPending}
                        onClick={() => {
                          if (window.confirm(t('confirmDeleteCharacter'))) {
                            characterDeleteMutation.mutate(character.id);
                          }
                        }}
                      >
                        {t('common:delete')}
                      </GlassButton>
                    </div>
                  </article>
                );
              })}
              {!charactersQuery.isLoading && characters.length === 0 ? (
                <p className="text-xs text-[var(--glass-text-tertiary)]">{t('emptyState')}</p>
              ) : null}
            </div>
          </GlassSurface>

          <GlassSurface>
            <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">{t('locations')}</h3>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {locations.map((location) => {
                const preview = getLocationPreview(location);
                return (
                  <article key={location.id} className="glass-list-row flex-col items-stretch gap-2 p-2">
                    <button
                      type="button"
                      className="block w-full text-left"
                      onClick={() => {
                        if (preview) {
                          setPreviewImageUrl(preview);
                        }
                      }}
                    >
                      <div className="mb-2 overflow-hidden rounded-lg bg-[var(--glass-bg-muted)]">
                        {preview ? (
                          <MediaImageWithLoading
                            src={preview}
                            alt={location.name}
                            containerClassName="h-28 w-full"
                            className="h-28 w-full object-cover"
                          />
                        ) : (
                          <div className="flex h-28 items-center justify-center text-xs text-[var(--glass-text-tertiary)]">
                            {t('emptyState')}
                          </div>
                        )}
                      </div>
                      <p className="text-sm font-semibold">{location.name}</p>
                      <p className="text-xs text-[var(--glass-text-tertiary)]">{location.images.length} images</p>
                    </button>
                    <div className="flex flex-wrap gap-1">
                      <GlassButton
                        size="sm"
                        variant="ghost"
                        onClick={() =>
                          setLocationEdit({
                            locationId: location.id,
                            name: location.name,
                            summary: location.summary ?? '',
                            originalName: location.name,
                            originalSummary: location.summary ?? '',
                          })
                        }
                      >
                        {t('common:edit')}
                      </GlassButton>
                      <GlassButton
                        size="sm"
                        variant="soft"
                        loading={regenerateMutation.isPending}
                        onClick={() => {
                          regenerateMutation.mutate({
                            type: 'location',
                            id: location.id,
                          });
                        }}
                      >
                        {t('regenerate')}
                      </GlassButton>
                      <GlassButton
                        size="sm"
                        variant="danger"
                        loading={locationDeleteMutation.isPending}
                        onClick={() => {
                          if (window.confirm(t('confirmDeleteLocation'))) {
                            locationDeleteMutation.mutate(location.id);
                          }
                        }}
                      >
                        {t('common:delete')}
                      </GlassButton>
                    </div>
                  </article>
                );
              })}
              {!locationsQuery.isLoading && locations.length === 0 ? (
                <p className="text-xs text-[var(--glass-text-tertiary)]">{t('emptyState')}</p>
              ) : null}
            </div>
          </GlassSurface>

          <GlassSurface>
            <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">{t('voices')}</h3>
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {voices.map((voice) => (
                <article key={voice.id} className="glass-list-row flex-col items-start gap-1">
                  <p className="text-sm font-semibold">{voice.name}</p>
                  <p className="text-xs text-[var(--glass-text-tertiary)]">{voice.voiceType}</p>
                  <div className="mt-2 flex gap-1">
                    <GlassButton
                      size="sm"
                      variant="danger"
                      loading={voiceDeleteMutation.isPending}
                      onClick={() => {
                        if (window.confirm(t('confirmDeleteVoice'))) {
                          voiceDeleteMutation.mutate(voice.id);
                        }
                      }}
                    >
                      {t('common:delete')}
                    </GlassButton>
                  </div>
                </article>
              ))}
              {!voicesQuery.isLoading && voices.length === 0 ? (
                <p className="text-xs text-[var(--glass-text-tertiary)]">{t('emptyState')}</p>
              ) : null}
            </div>
          </GlassSurface>
        </div>
      </div>

      <GlassModalShell
        open={folderModalOpen}
        onClose={() => {
          setFolderModalOpen(false);
          setEditingFolderId(null);
          setFolderForm(EMPTY_FOLDER);
        }}
        title={editingFolderId ? t('editFolder') : t('newFolder')}
        size="sm"
      >
        <form className="space-y-4" onSubmit={onCreateFolder}>
          <GlassField id="folder-name" label={t('folderName')} required>
            <GlassInput
              id="folder-name"
              value={folderForm.name}
              placeholder={t('folderNamePlaceholder')}
              onChange={(event) => setFolderForm({ name: event.target.value })}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setFolderModalOpen(false)}>
              {t('cancel')}
            </GlassButton>
            <GlassButton
              type="submit"
              variant="primary"
              loading={folderCreateMutation.isPending || folderUpdateMutation.isPending}
            >
              {editingFolderId ? t('save') : t('create')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell open={characterModalOpen} onClose={() => setCharacterModalOpen(false)} title={t('addCharacter')}>
        <form className="space-y-4" onSubmit={onCreateCharacter}>
          <GlassField id="character-name" label={t('modal.nameLabel')} required>
            <GlassInput
              id="character-name"
              value={characterForm.name}
              placeholder={t('modal.namePlaceholder')}
              onChange={(event) => setCharacterForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </GlassField>
          <GlassField id="character-description" label={t('modal.descLabel')}>
            <GlassTextarea
              id="character-description"
              rows={4}
              value={characterForm.description}
              placeholder={t('modal.descPlaceholder')}
              onChange={(event) => setCharacterForm((prev) => ({ ...prev, description: event.target.value }))}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setCharacterModalOpen(false)}>
              {t('cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={characterMutation.isPending}>
              {t('create')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell open={locationModalOpen} onClose={() => setLocationModalOpen(false)} title={t('addLocation')}>
        <form className="space-y-4" onSubmit={onCreateLocation}>
          <GlassField id="location-name" label={t('modal.locationNameLabel')} required>
            <GlassInput
              id="location-name"
              value={locationForm.name}
              placeholder={t('modal.locationNamePlaceholder')}
              onChange={(event) => setLocationForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </GlassField>
          <GlassField id="location-summary" label={t('modal.locationSummaryLabel')}>
            <GlassTextarea
              id="location-summary"
              rows={4}
              value={locationForm.description}
              placeholder={t('modal.locationSummaryPlaceholder')}
              onChange={(event) => setLocationForm((prev) => ({ ...prev, description: event.target.value }))}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setLocationModalOpen(false)}>
              {t('cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={locationMutation.isPending}>
              {t('create')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell open={voiceModalOpen} onClose={() => setVoiceModalOpen(false)} title={t('addVoice')}>
        <form className="space-y-4" onSubmit={onCreateVoice}>
          <GlassField id="voice-name" label={t('voiceName')} required>
            <GlassInput
              id="voice-name"
              value={voiceForm.name}
              placeholder={t('voiceNamePlaceholder')}
              onChange={(event) => setVoiceForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </GlassField>
          <GlassField id="voice-description" label={t('modal.descLabel')}>
            <GlassTextarea
              id="voice-description"
              rows={3}
              value={voiceForm.description}
              onChange={(event) => setVoiceForm((prev) => ({ ...prev, description: event.target.value }))}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setVoiceModalOpen(false)}>
              {t('cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={voiceCreateMutation.isPending}>
              {t('create')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell
        open={characterEdit !== null}
        onClose={() => setCharacterEdit(null)}
        title={t('common:edit')}
      >
        {characterEdit ? (
          <div className="space-y-4">
            <GlassField id="character-edit-name" label={t('modal.nameLabel')} required>
              <GlassInput
                id="character-edit-name"
                value={characterEdit.name}
                onChange={(event) =>
                  setCharacterEdit((previous) =>
                    previous
                      ? {
                          ...previous,
                          name: event.target.value,
                        }
                      : previous,
                  )
                }
              />
            </GlassField>
            <GlassField id="character-edit-description" label={t('modal.descLabel')} required>
              <GlassTextarea
                id="character-edit-description"
                rows={6}
                value={characterEdit.description}
                onChange={(event) =>
                  setCharacterEdit((previous) =>
                    previous
                      ? {
                          ...previous,
                          description: event.target.value,
                        }
                      : previous,
                  )
                }
              />
            </GlassField>
            <div className="flex justify-end gap-2">
              <GlassButton type="button" variant="ghost" onClick={() => setCharacterEdit(null)}>
                {t('cancel')}
              </GlassButton>
              <GlassButton
                type="button"
                variant="secondary"
                loading={isCharacterSaving}
                onClick={() => {
                  void submitCharacterEdit(false);
                }}
              >
                {t('save')}
              </GlassButton>
              <GlassButton
                type="button"
                variant="primary"
                loading={isCharacterSaving}
                onClick={() => {
                  void submitCharacterEdit(true);
                }}
              >
                {t('regenerate')}
              </GlassButton>
            </div>
          </div>
        ) : null}
      </GlassModalShell>

      <GlassModalShell
        open={locationEdit !== null}
        onClose={() => setLocationEdit(null)}
        title={t('common:edit')}
      >
        {locationEdit ? (
          <div className="space-y-4">
            <GlassField id="location-edit-name" label={t('modal.locationNameLabel')} required>
              <GlassInput
                id="location-edit-name"
                value={locationEdit.name}
                onChange={(event) =>
                  setLocationEdit((previous) =>
                    previous
                      ? {
                          ...previous,
                          name: event.target.value,
                        }
                      : previous,
                  )
                }
              />
            </GlassField>
            <GlassField id="location-edit-summary" label={t('modal.locationSummaryLabel')} required>
              <GlassTextarea
                id="location-edit-summary"
                rows={6}
                value={locationEdit.summary}
                onChange={(event) =>
                  setLocationEdit((previous) =>
                    previous
                      ? {
                          ...previous,
                          summary: event.target.value,
                        }
                      : previous,
                  )
                }
              />
            </GlassField>
            <div className="flex justify-end gap-2">
              <GlassButton type="button" variant="ghost" onClick={() => setLocationEdit(null)}>
                {t('cancel')}
              </GlassButton>
              <GlassButton
                type="button"
                variant="secondary"
                loading={isLocationSaving}
                onClick={() => {
                  void submitLocationEdit(false);
                }}
              >
                {t('save')}
              </GlassButton>
              <GlassButton
                type="button"
                variant="primary"
                loading={isLocationSaving}
                onClick={() => {
                  void submitLocationEdit(true);
                }}
              >
                {t('regenerate')}
              </GlassButton>
            </div>
          </div>
        ) : null}
      </GlassModalShell>

      <GlassModalShell
        open={voiceBind !== null}
        onClose={() => setVoiceBind(null)}
        title={t('voicePickerTitle')}
        size="sm"
      >
        {voiceBind ? (
          <div className="space-y-4">
            <p className="text-sm text-[var(--glass-text-secondary)]">{voiceBind.characterName}</p>
            <GlassField id="voice-picker-select" label={t('voiceName')} required>
              <select
                id="voice-picker-select"
                className="glass-input-base h-10 w-full px-3"
                value={voiceBind.selectedVoiceId}
                onChange={(event) =>
                  setVoiceBind((previous) =>
                    previous
                      ? {
                          ...previous,
                          selectedVoiceId: event.target.value,
                        }
                      : previous,
                  )
                }
              >
                <option value="">{t('voicePickerEmpty')}</option>
                {allVoices.map((voice) => (
                  <option key={voice.id} value={voice.id}>
                    {voice.name}
                  </option>
                ))}
              </select>
            </GlassField>
            <div className="flex justify-end gap-2">
              <GlassButton type="button" variant="ghost" onClick={() => setVoiceBind(null)}>
                {t('cancel')}
              </GlassButton>
              <GlassButton
                type="button"
                variant="primary"
                loading={voiceBindMutation.isPending}
                disabled={!voiceBind.selectedVoiceId}
                onClick={() => {
                  if (!voiceBind.selectedVoiceId) {
                    return;
                  }
                  voiceBindMutation.mutate({
                    characterId: voiceBind.characterId,
                    voiceId: voiceBind.selectedVoiceId,
                  });
                }}
              >
                {t('voicePickerConfirm')}
              </GlassButton>
            </div>
          </div>
        ) : null}
      </GlassModalShell>

      <ImagePreviewModal imageUrl={previewImageUrl} onClose={() => setPreviewImageUrl(null)} />
    </main>
  );
}
