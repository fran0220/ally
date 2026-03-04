import { type FormEvent, type KeyboardEvent, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import {
  type ProjectSummary,
  createProject,
  deleteProject,
  listProjects,
  updateProject,
} from '../../api/projects';
import {
  GlassButton,
  GlassField,
  GlassInput,
  GlassModalShell,
  GlassSurface,
  GlassTextarea,
} from '../../components/ui/primitives';
import { queryKeys } from '../../lib/query-keys';

const PAGE_SIZE = 12;

function formatDate(value: string): string {
  return new Date(value).toLocaleString();
}

interface ProjectFormState {
  name: string;
  description: string;
}

const EMPTY_FORM: ProjectFormState = { name: '', description: '' };

export function WorkspaceList() {
  const { t } = useTranslation(['workspace', 'common']);
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  const [page, setPage] = useState(1);
  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');

  const [createOpen, setCreateOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<ProjectSummary | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProjectSummary | null>(null);
  const [form, setForm] = useState<ProjectFormState>(EMPTY_FORM);

  const projectsQuery = useQuery({
    queryKey: queryKeys.projects.list(page, PAGE_SIZE, search),
    queryFn: () => listProjects(page, PAGE_SIZE, search),
  });

  const createMutation = useMutation({
    mutationFn: createProject,
    onSuccess: () => {
      setCreateOpen(false);
      setForm(EMPTY_FORM);
      void queryClient.invalidateQueries({ queryKey: ['projects'] });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ projectId, name, description }: { projectId: string; name: string; description: string }) =>
      updateProject(projectId, { name, description }),
    onSuccess: () => {
      setEditTarget(null);
      setForm(EMPTY_FORM);
      void queryClient.invalidateQueries({ queryKey: ['projects'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (projectId: string) => deleteProject(projectId),
    onSuccess: () => {
      setDeleteTarget(null);
      void queryClient.invalidateQueries({ queryKey: ['projects'] });
    },
  });

  const isSubmitting = createMutation.isPending || updateMutation.isPending;
  const projects = projectsQuery.data?.projects ?? [];
  const pagination = projectsQuery.data?.pagination;

  const pageNumbers = useMemo(() => {
    if (!pagination || pagination.totalPages <= 1) {
      return [];
    }
    const start = Math.max(1, pagination.page - 2);
    const end = Math.min(pagination.totalPages, pagination.page + 2);
    return Array.from({ length: end - start + 1 }, (_, index) => start + index);
  }, [pagination]);

  function openCreateModal() {
    setForm(EMPTY_FORM);
    setCreateOpen(true);
  }

  function openEditModal(project: ProjectSummary) {
    setEditTarget(project);
    setForm({ name: project.name, description: project.description ?? '' });
  }

  function submitSearch() {
    setPage(1);
    setSearch(searchInput.trim());
  }

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form.name.trim()) {
      return;
    }
    await createMutation.mutateAsync({
      name: form.name.trim(),
      description: form.description.trim() || undefined,
    });
  }

  async function handleEdit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editTarget || !form.name.trim()) {
      return;
    }
    await updateMutation.mutateAsync({
      projectId: editTarget.id,
      name: form.name.trim(),
      description: form.description.trim(),
    });
  }

  function openProject(projectId: string) {
    navigate(`/workspace/${projectId}`);
  }

  function handleProjectCardKeyDown(event: KeyboardEvent<HTMLDivElement>, projectId: string) {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      openProject(projectId);
    }
  }

  return (
    <main className="page-shell py-8 md:py-10">
      <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="glass-page-title">{t('workspace:title')}</h1>
          <p className="glass-page-subtitle">{t('workspace:subtitle')}</p>
        </div>
        <GlassButton variant="primary" onClick={openCreateModal}>
          + {t('workspace:newProject')}
        </GlassButton>
      </header>

      <GlassSurface className="mb-6" density="compact">
        <div className="flex flex-wrap items-end gap-3">
          <GlassField className="min-w-72 flex-1" id="workspace-search" label={t('workspace:searchPlaceholder')}>
            <GlassInput
              id="workspace-search"
              value={searchInput}
              placeholder={t('workspace:searchPlaceholder')}
              onChange={(event) => setSearchInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  submitSearch();
                }
              }}
            />
          </GlassField>
          <GlassButton variant="secondary" onClick={submitSearch}>
            {t('workspace:searchButton')}
          </GlassButton>
          {search ? (
            <GlassButton
              variant="ghost"
              onClick={() => {
                setSearch('');
                setSearchInput('');
                setPage(1);
              }}
            >
              {t('workspace:clearButton')}
            </GlassButton>
          ) : null}
        </div>
      </GlassSurface>

      {projectsQuery.error instanceof Error ? (
        <p className="mb-6 text-sm text-[var(--glass-tone-danger-fg)]">{projectsQuery.error.message}</p>
      ) : null}

      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        <button
          type="button"
          className="glass-surface group flex min-h-48 cursor-pointer flex-col items-center justify-center border border-dashed border-[var(--glass-stroke-focus)] p-6 text-center"
          onClick={openCreateModal}
        >
          <span className="mb-2 inline-flex h-12 w-12 items-center justify-center rounded-full bg-[var(--glass-tone-info-bg)] text-2xl text-[var(--glass-tone-info-fg)] transition-transform group-hover:scale-110">
            +
          </span>
          <p className="text-sm font-medium text-[var(--glass-text-secondary)]">{t('workspace:newProject')}</p>
        </button>

        {projectsQuery.isLoading
          ? Array.from({ length: 5 }).map((_, index) => (
              <div key={`skeleton-${index}`} className="glass-surface min-h-48 animate-pulse p-6">
                <div className="h-4 w-2/3 rounded bg-white/65" />
                <div className="mt-3 h-3 rounded bg-white/55" />
                <div className="mt-2 h-3 w-4/5 rounded bg-white/55" />
              </div>
            ))
          : projects.map((project) => (
              <GlassSurface
                key={project.id}
                className="min-h-48 cursor-pointer"
                interactive
                role="link"
                tabIndex={0}
                onClick={() => openProject(project.id)}
                onKeyDown={(event) => handleProjectCardKeyDown(event, project.id)}
              >
                <div className="flex h-full flex-col">
                  <div className="mb-3 flex items-start justify-between gap-3">
                    <span className="text-lg font-semibold text-[var(--glass-text-primary)]">
                      {project.name}
                    </span>
                    <div className="flex gap-1">
                      <GlassButton
                        variant="ghost"
                        size="sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          openEditModal(project);
                        }}
                      >
                        {t('common:edit')}
                      </GlassButton>
                      <GlassButton
                        variant="danger"
                        size="sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          setDeleteTarget(project);
                        }}
                      >
                        {t('common:delete')}
                      </GlassButton>
                    </div>
                  </div>

                  <p className="line-clamp-3 flex-1 text-sm text-[var(--glass-text-secondary)]">
                    {project.description || t('workspace:noContent')}
                  </p>

                  <div className="mt-4 text-xs text-[var(--glass-text-tertiary)]">{formatDate(project.updatedAt)}</div>
                </div>
              </GlassSurface>
            ))}
      </section>

      {!projectsQuery.isLoading && projects.length === 0 ? (
        <GlassSurface className="mt-6 text-center">
          <p className="text-base font-medium text-[var(--glass-text-primary)]">
            {search ? t('workspace:noResults') : t('workspace:noProjects')}
          </p>
          <p className="mt-2 text-sm text-[var(--glass-text-secondary)]">
            {search ? t('workspace:noResultsDesc') : t('workspace:noProjectsDesc')}
          </p>
          {!search ? (
            <GlassButton className="mt-4" variant="primary" onClick={openCreateModal}>
              {t('workspace:newProject')}
            </GlassButton>
          ) : null}
        </GlassSurface>
      ) : null}

      {pagination && pagination.totalPages > 1 ? (
        <footer className="mt-6 flex flex-wrap items-center justify-center gap-2">
          <GlassButton
            size="sm"
            variant="soft"
            disabled={pagination.page <= 1}
            onClick={() => setPage((previous) => Math.max(1, previous - 1))}
          >
            {t('common:previous')}
          </GlassButton>
          {pageNumbers.map((item) => (
            <GlassButton
              key={item}
              size="sm"
              variant={item === pagination.page ? 'primary' : 'soft'}
              onClick={() => setPage(item)}
            >
              {String(item)}
            </GlassButton>
          ))}
          <GlassButton
            size="sm"
            variant="soft"
            disabled={pagination.page >= pagination.totalPages}
            onClick={() => setPage((previous) => Math.min(pagination.totalPages, previous + 1))}
          >
            {t('common:next')}
          </GlassButton>
          <span className="ml-2 text-xs text-[var(--glass-text-tertiary)]">
            {t('workspace:totalProjects', { count: pagination.total })}
          </span>
        </footer>
      ) : null}

      <GlassModalShell
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          setForm(EMPTY_FORM);
        }}
        title={t('workspace:createProject')}
      >
        <form className="space-y-4" onSubmit={handleCreate}>
          <GlassField id="project-create-name" label={t('workspace:projectName')} required>
            <GlassInput
              id="project-create-name"
              value={form.name}
              maxLength={100}
              placeholder={t('workspace:projectNamePlaceholder')}
              onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </GlassField>
          <GlassField id="project-create-description" label={t('workspace:projectDescription')}>
            <GlassTextarea
              id="project-create-description"
              rows={3}
              maxLength={500}
              placeholder={t('workspace:projectDescriptionPlaceholder')}
              value={form.description}
              onChange={(event) => setForm((prev) => ({ ...prev, description: event.target.value }))}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setCreateOpen(false)}>
              {t('common:cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={isSubmitting}>
              {isSubmitting ? t('workspace:creating') : t('workspace:createProject')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell
        open={editTarget !== null}
        onClose={() => {
          setEditTarget(null);
          setForm(EMPTY_FORM);
        }}
        title={t('workspace:editProject')}
      >
        <form className="space-y-4" onSubmit={handleEdit}>
          <GlassField id="project-edit-name" label={t('workspace:projectName')} required>
            <GlassInput
              id="project-edit-name"
              value={form.name}
              maxLength={100}
              onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </GlassField>
          <GlassField id="project-edit-description" label={t('workspace:projectDescription')}>
            <GlassTextarea
              id="project-edit-description"
              rows={3}
              maxLength={500}
              value={form.description}
              onChange={(event) => setForm((prev) => ({ ...prev, description: event.target.value }))}
            />
          </GlassField>
          <div className="flex justify-end gap-2">
            <GlassButton type="button" variant="ghost" onClick={() => setEditTarget(null)}>
              {t('common:cancel')}
            </GlassButton>
            <GlassButton type="submit" variant="primary" loading={isSubmitting}>
              {isSubmitting ? t('workspace:saving') : t('common:save')}
            </GlassButton>
          </div>
        </form>
      </GlassModalShell>

      <GlassModalShell
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        title={t('workspace:deleteProject')}
        description={
          deleteTarget ? t('workspace:deleteConfirm', { name: deleteTarget.name }) : ''
        }
        size="sm"
      >
        <div className="flex justify-end gap-2">
          <GlassButton type="button" variant="ghost" onClick={() => setDeleteTarget(null)}>
            {t('common:cancel')}
          </GlassButton>
          <GlassButton
            type="button"
            variant="danger"
            loading={deleteMutation.isPending}
            onClick={() => {
              if (deleteTarget) {
                deleteMutation.mutate(deleteTarget.id);
              }
            }}
          >
            {t('common:delete')}
          </GlassButton>
        </div>
      </GlassModalShell>
    </main>
  );
}
