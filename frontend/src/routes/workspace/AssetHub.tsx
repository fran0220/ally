import { type FormEvent, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

import {
  createAssetCharacter,
  createAssetFolder,
  createAssetLocation,
  createAssetVoice,
  deleteAssetFolder,
  listAssetCharacters,
  listAssetFolders,
  listAssetLocations,
  listAssetVoices,
} from '../../api/asset-hub';
import { useTaskSse } from '../../hooks/use-task-sse';
import { queryKeys } from '../../lib/query-keys';
import { GlassButton, GlassField, GlassModalShell, GlassSurface, GlassInput, GlassTextarea, GlassChip } from '../../components/ui/primitives';

interface FolderFormState {
  name: string;
}

interface AssetFormState {
  name: string;
  description: string;
}

const EMPTY_FOLDER: FolderFormState = { name: '' };
const EMPTY_ASSET: AssetFormState = { name: '', description: '' };

export function AssetHub() {
  const { t } = useTranslation('assetHub');
  const queryClient = useQueryClient();

  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null);
  const [folderForm, setFolderForm] = useState<FolderFormState>(EMPTY_FOLDER);
  const [characterForm, setCharacterForm] = useState<AssetFormState>(EMPTY_ASSET);
  const [locationForm, setLocationForm] = useState<AssetFormState>(EMPTY_ASSET);
  const [voiceForm, setVoiceForm] = useState<AssetFormState>(EMPTY_ASSET);

  const [folderModalOpen, setFolderModalOpen] = useState(false);
  const [characterModalOpen, setCharacterModalOpen] = useState(false);
  const [locationModalOpen, setLocationModalOpen] = useState(false);
  const [voiceModalOpen, setVoiceModalOpen] = useState(false);

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

  const folderMutation = useMutation({
    mutationFn: (name: string) => createAssetFolder(name),
    onSuccess: () => {
      setFolderModalOpen(false);
      setFolderForm(EMPTY_FOLDER);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub'] });
    },
  });
  const folderDeleteMutation = useMutation({
    mutationFn: (folderId: string) => deleteAssetFolder(folderId),
    onSuccess: () => {
      if (selectedFolderId) {
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
      void queryClient.invalidateQueries({ queryKey: ['asset-hub', 'characters'] });
    },
  });
  const locationMutation = useMutation({
    mutationFn: () =>
      createAssetLocation({
        name: locationForm.name.trim(),
        summary: locationForm.description.trim(),
        folderId: selectedFolderId,
      }),
    onSuccess: () => {
      setLocationModalOpen(false);
      setLocationForm(EMPTY_ASSET);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub', 'locations'] });
    },
  });
  const voiceMutation = useMutation({
    mutationFn: () =>
      createAssetVoice({
        name: voiceForm.name.trim(),
        description: voiceForm.description.trim(),
        folderId: selectedFolderId,
      }),
    onSuccess: () => {
      setVoiceModalOpen(false);
      setVoiceForm(EMPTY_ASSET);
      void queryClient.invalidateQueries({ queryKey: ['asset-hub', 'voices'] });
    },
  });

  const { connected, events } = useTaskSse({ projectId: 'global-asset-hub', enabled: true });

  const folders = foldersQuery.data?.folders ?? [];
  const characters = charactersQuery.data?.characters ?? [];
  const locations = locationsQuery.data?.locations ?? [];
  const voices = voicesQuery.data?.voices ?? [];

  const activeFolderName = useMemo(() => {
    if (!selectedFolderId) {
      return t('allAssets');
    }
    return folders.find((folder) => folder.id === selectedFolderId)?.name ?? t('allAssets');
  }, [folders, selectedFolderId, t]);

  async function onCreateFolder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!folderForm.name.trim()) {
      return;
    }
    await folderMutation.mutateAsync(folderForm.name.trim());
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
    await voiceMutation.mutateAsync();
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
            <GlassButton size="sm" variant="soft" onClick={() => setFolderModalOpen(true)}>
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
              <div key={folder.id} className="glass-list-row">
                <button
                  type="button"
                  className="flex-1 text-left"
                  onClick={() => setSelectedFolderId(folder.id)}
                >
                  {folder.name}
                </button>
                <GlassButton
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    if (window.confirm(t('confirmDeleteFolder'))) {
                      folderDeleteMutation.mutate(folder.id);
                    }
                  }}
                >
                  x
                </GlassButton>
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
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {characters.map((character) => (
                <article key={character.id} className="glass-list-row flex-col items-start gap-1">
                  <p className="text-sm font-semibold">{character.name}</p>
                  <p className="text-xs text-[var(--glass-text-tertiary)]">
                    {character.appearances.length} appearances
                  </p>
                </article>
              ))}
              {!charactersQuery.isLoading && characters.length === 0 ? (
                <p className="text-xs text-[var(--glass-text-tertiary)]">{t('emptyState')}</p>
              ) : null}
            </div>
          </GlassSurface>

          <GlassSurface>
            <h3 className="mb-3 text-sm font-semibold text-[var(--glass-text-secondary)]">{t('locations')}</h3>
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {locations.map((location) => (
                <article key={location.id} className="glass-list-row flex-col items-start gap-1">
                  <p className="text-sm font-semibold">{location.name}</p>
                  <p className="text-xs text-[var(--glass-text-tertiary)]">{location.images.length} images</p>
                </article>
              ))}
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
                </article>
              ))}
              {!voicesQuery.isLoading && voices.length === 0 ? (
                <p className="text-xs text-[var(--glass-text-tertiary)]">{t('emptyState')}</p>
              ) : null}
            </div>
          </GlassSurface>
        </div>
      </div>

      <GlassModalShell open={folderModalOpen} onClose={() => setFolderModalOpen(false)} title={t('newFolder')} size="sm">
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
            <GlassButton type="submit" variant="primary" loading={folderMutation.isPending}>
              {t('create')}
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
          <GlassField id="voice-description" label={t('description')}>
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
            <GlassButton type="submit" variant="primary" loading={voiceMutation.isPending}>
              {t('create')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>
    </main>
  );
}
