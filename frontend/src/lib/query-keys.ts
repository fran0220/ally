export const queryKeys = {
  projects: {
    list: (page: number, pageSize: number, search: string) =>
      ['projects', 'list', page, pageSize, search] as const,
    detail: (projectId: string) => ['projects', 'detail', projectId] as const,
    data: (projectId: string) => ['projects', 'data', projectId] as const,
    assets: (projectId: string) => ['projects', 'assets', projectId] as const,
  },
  novel: {
    root: (projectId: string) => ['novel', 'root', projectId] as const,
    episodes: (projectId: string) => ['novel', 'episodes', projectId] as const,
    episode: (projectId: string, episodeId: string) => ['novel', 'episode', projectId, episodeId] as const,
    storyboards: (projectId: string, episodeId: string) =>
      ['novel', 'storyboards', projectId, episodeId] as const,
    editor: (projectId: string, episodeId: string) => ['novel', 'editor', projectId, episodeId] as const,
  },
  assetHub: {
    folders: () => ['asset-hub', 'folders'] as const,
    characters: (folderId: string | null) => ['asset-hub', 'characters', folderId ?? 'all'] as const,
    locations: (folderId: string | null) => ['asset-hub', 'locations', folderId ?? 'all'] as const,
    voices: (folderId: string | null) => ['asset-hub', 'voices', folderId ?? 'all'] as const,
  },
  admin: {
    aiConfig: () => ['admin', 'ai-config'] as const,
  },
  user: {
    preference: () => ['user', 'preference'] as const,
    models: () => ['user', 'models'] as const,
  },
  tasks: {
    list: (projectId: string, episodeId?: string | null) =>
      ['tasks', 'list', projectId, episodeId ?? 'all'] as const,
  },
} as const;
