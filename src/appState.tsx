import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ApiSource,
  AppSettings,
  CacheStats,
  defaultSettings,
  Library,
  Mood,
  moodQueries,
  ImportResult,
  SyncAuthContext,
  SupabaseSyncApplyResult,
  SupabaseSyncStatus,
  WallpaperLayoutPreference,
  ViewName,
  Wallpaper,
  WallpaperQualityReport,
} from "./types";
import { logAppAction } from "./appLog";
import { sourceSelectionSearch } from "./searchFlow";
import { pickRandomMoodQuery, pickRandomWallpaper } from "./wallpaperSelection";

export interface AppState {
  activeView: ViewName;
  settings: AppSettings;
  currentWallpaper: Wallpaper | null;
  library: Library;
  cacheStats: CacheStats;
  query: string;
  source: ApiSource;
  mood: Mood;
  results: Wallpaper[];
  page: number;
  busy: string | null;
  notice: string;
}

export interface AppActions {
  setActiveView: (view: ViewName) => void;
  setQuery: (query: string) => void;
  searchWallpapers: (
    nextPage?: number,
    nextQuery?: string,
    nextSource?: ApiSource,
    nextMood?: Mood | null,
  ) => Promise<void>;
  changeSource: (source: ApiSource) => Promise<void>;
  setWallpaper: (wallpaper: Wallpaper) => Promise<void>;
  setWallpaperWithLayout: (
    wallpaper: Wallpaper,
    layout: WallpaperLayoutPreference,
  ) => Promise<void>;
  setLockScreenWallpaper: (wallpaper: Wallpaper) => Promise<void>;
  assessWallpaperQuality: (
    wallpaper: Wallpaper,
  ) => Promise<WallpaperQualityReport | null>;
  saveFavorite: (wallpaper: Wallpaper) => Promise<void>;
  createPlaylist: (name: string) => Promise<void>;
  deletePlaylist: (playlistId: string) => Promise<void>;
  addWallpaperToPlaylist: (
    playlistId: string,
    wallpaper: Wallpaper,
  ) => Promise<void>;
  removeWallpaperFromPlaylist: (
    playlistId: string,
    wallpaperId: string,
  ) => Promise<void>;
  importLocalFolder: (folderPath: string) => Promise<ImportResult | null>;
  exportBackup: (targetPath: string) => Promise<string | null>;
  importBackup: (sourcePath: string) => Promise<void>;
  testSupabaseSync: (
    authContext?: SyncAuthContext | null,
  ) => Promise<SupabaseSyncStatus | null>;
  pushSupabaseSync: (
    authContext?: SyncAuthContext | null,
  ) => Promise<SupabaseSyncStatus | null>;
  pullSupabaseSync: (
    authContext?: SyncAuthContext | null,
  ) => Promise<SupabaseSyncStatus | null>;
  runAutoCleanup: () => Promise<void>;
  applyNextWallpaper: () => Promise<void>;
  toggleAutoChangePause: () => Promise<boolean | null>;
  applyRandomWallpaper: () => Promise<void>;
  applyMood: (mood: Mood) => Promise<void>;
  applyTopic: (query: string) => Promise<void>;
  applyNextFromMood: () => Promise<void>;
  clearWallpaperCache: () => Promise<void>;
  clearLibrary: () => Promise<void>;
  deleteWallpaper: (wallpaper: Wallpaper) => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
}

export interface AppStateValue extends AppState {
  hasAnyKey: boolean;
  favoriteIds: Set<string>;
  actions: AppActions;
}

export type AppAction =
  | { type: "activeViewChanged"; activeView: ViewName }
  | { type: "settingsLoaded"; settings: AppSettings }
  | { type: "currentWallpaperChanged"; wallpaper: Wallpaper | null }
  | { type: "libraryLoaded"; library: Library }
  | { type: "cacheStatsLoaded"; cacheStats: CacheStats }
  | { type: "queryChanged"; query: string }
  | { type: "sourceChanged"; source: ApiSource }
  | { type: "moodChanged"; mood: Mood }
  | { type: "busyChanged"; busy: string | null }
  | { type: "noticeChanged"; notice: string }
  | { type: "searchLoaded"; page: number; wallpapers: Wallpaper[] };

export const initialAppState: AppState = {
  activeView: "home",
  settings: defaultSettings,
  currentWallpaper: null,
  library: {
    favorites: [],
    downloaded: [],
    playlists: [],
  },
  cacheStats: {
    bytes: 0,
    files: 0,
  },
  query: "nature",
  source: "all",
  mood: "nature",
  results: [],
  page: 1,
  busy: null,
  notice: "",
};

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "activeViewChanged":
      return { ...state, activeView: action.activeView };
    case "settingsLoaded":
      return { ...state, settings: action.settings };
    case "currentWallpaperChanged":
      return { ...state, currentWallpaper: action.wallpaper };
    case "libraryLoaded":
      return { ...state, library: action.library };
    case "cacheStatsLoaded":
      return { ...state, cacheStats: action.cacheStats };
    case "queryChanged":
      return { ...state, query: action.query };
    case "sourceChanged":
      return { ...state, source: action.source };
    case "moodChanged":
      return { ...state, mood: action.mood };
    case "busyChanged":
      return { ...state, busy: action.busy };
    case "noticeChanged":
      return { ...state, notice: action.notice };
    case "searchLoaded":
      return {
        ...state,
        activeView: "search",
        page: action.page,
        results:
          action.page === 1
            ? action.wallpapers
            : [...state.results, ...action.wallpapers],
      };
  }
}

export const AppStateContext = createContext<AppStateValue | null>(null);

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface AppStateProviderProps {
  children: ReactNode;
}

export function AppStateProvider({ children }: AppStateProviderProps) {
  const [state, dispatch] = useReducer(appReducer, initialAppState);

  const refreshLibrary = useCallback(async () => {
    const nextLibrary = await invoke<Library>("list_library");
    dispatch({ type: "libraryLoaded", library: nextLibrary });
  }, []);

  const refreshCacheStats = useCallback(async () => {
    const stats = await invoke<CacheStats>("cache_stats");
    dispatch({ type: "cacheStatsLoaded", cacheStats: stats });
  }, []);

  useEffect(() => {
    async function boot() {
      if (!isTauriRuntime()) {
        return;
      }

      try {
        void logAppAction("app.boot.start", "App boot state load started.");
        const loaded = await invoke<AppSettings>("get_settings");
        dispatch({ type: "settingsLoaded", settings: loaded });
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
        void logAppAction("app.boot.success", "App boot state loaded.");
      } catch (error) {
        dispatch({ type: "noticeChanged", notice: String(error) });
        void logAppAction(
          "app.boot.error",
          "App boot state load failed.",
          { error: String(error) },
          "error",
        );
      }
    }

    void boot();
  }, [refreshCacheStats, refreshLibrary]);

  const runWithStatus = useCallback(
    async <T,>(
      label: string,
      action: () => Promise<T>,
      done?: string,
    ): Promise<T | null> => {
      dispatch({ type: "busyChanged", busy: label });
      dispatch({ type: "noticeChanged", notice: "" });
      void logAppAction("action.start", "UI action started.", { label });
      if (!isTauriRuntime()) {
        dispatch({
          type: "noticeChanged",
          notice: "Open the desktop app to use wallpaper actions.",
        });
        void logAppAction(
          "action.unavailable",
          "UI action skipped outside desktop runtime.",
          { label },
          "warn",
        );
        dispatch({ type: "busyChanged", busy: null });
        return null;
      }

      try {
        const value = await action();
        if (done) {
          dispatch({ type: "noticeChanged", notice: done });
        }
        void logAppAction("action.success", "UI action completed.", {
          label,
          notice: done ?? "",
        });
        return value;
      } catch (error) {
        dispatch({ type: "noticeChanged", notice: String(error) });
        void logAppAction(
          "action.error",
          "UI action failed.",
          { label, error: String(error) },
          "error",
        );
        return null;
      } finally {
        dispatch({ type: "busyChanged", busy: null });
      }
    },
    [],
  );

  const saveSettings = useCallback(
    async (nextSettings: AppSettings) => {
      void logAppAction("settings.save.request", "Settings save requested.", {
        autoChangeMinutes: nextSettings.autoChangeMinutes,
        resolution: nextSettings.resolution,
        wallpaperLayout: nextSettings.wallpaperLayout,
        runInBackground: nextSettings.runInBackground,
        launchAtStartup: nextSettings.launchAtStartup,
        globalHotkeysEnabled: nextSettings.globalHotkeysEnabled,
        supabaseEnabled: nextSettings.supabaseSync.enabled,
        clerkEnabled: nextSettings.clerkAuth.enabled,
      });
      const saved = await runWithStatus(
        "settings",
        () => invoke<AppSettings>("save_settings", { settings: nextSettings }),
        "Settings saved.",
      );
      if (saved) {
        dispatch({ type: "settingsLoaded", settings: saved });
      }
    },
    [runWithStatus],
  );

  const searchWallpapers = useCallback(
    async (
      nextPage = 1,
      nextQuery = state.query,
      nextSource = state.source,
      nextMood: Mood | null = null,
    ) => {
      void logAppAction("search.request", "Wallpaper search requested.", {
        query: nextQuery,
        page: nextPage,
        source: nextSource,
        mood: nextMood,
      });
      const wallpapers = await runWithStatus("search", () =>
        invoke<Wallpaper[]>("search_wallpapers", {
          query: nextQuery,
          page: nextPage,
          source: nextSource,
        }),
      );
      if (!wallpapers) {
        return;
      }

      const nextWallpapers = nextMood
        ? wallpapers.map((wallpaper) => ({ ...wallpaper, mood: nextMood }))
        : wallpapers;
      dispatch({
        type: "searchLoaded",
        page: nextPage,
        wallpapers: nextWallpapers,
      });
    },
    [runWithStatus, state.query, state.source],
  );

  const changeSource = useCallback(
    async (nextSource: ApiSource) => {
      void logAppAction("search.source.change", "Wallpaper source changed.", {
        from: state.source,
        to: nextSource,
      });
      dispatch({ type: "sourceChanged", source: nextSource });
      const request = sourceSelectionSearch(state.query, nextSource);
      await searchWallpapers(
        request.nextPage,
        request.nextQuery,
        request.nextSource,
      );
    },
    [searchWallpapers, state.query],
  );

  const shouldApplyAfterQualityWarning = useCallback(
    async (wallpaper: Wallpaper) => {
      if (state.settings.qualityGuardMode !== "warn" || !isTauriRuntime()) {
        return true;
      }

      try {
        void logAppAction("quality.warning.check", "Quality warning check started.", {
          wallpaperId: wallpaper.id,
          source: wallpaper.source,
        });
        const report = await invoke<WallpaperQualityReport>(
          "assess_wallpaper_quality",
          { wallpaper },
        );
        if (report.warnings.length === 0) {
          void logAppAction("quality.warning.ok", "Quality warning check passed.", {
            wallpaperId: wallpaper.id,
          });
          return true;
        }
        const accepted = window.confirm(
          `Apply anyway?\n\n${report.warnings.join("\n")}`,
        );
        void logAppAction(
          accepted ? "quality.warning.accepted" : "quality.warning.rejected",
          accepted
            ? "User accepted quality warning."
            : "User rejected quality warning.",
          { wallpaperId: wallpaper.id, warnings: report.warnings },
          accepted ? "warn" : "info",
        );
        return accepted;
      } catch (error) {
        dispatch({ type: "noticeChanged", notice: String(error) });
        void logAppAction(
          "quality.warning.error",
          "Quality warning check failed.",
          { wallpaperId: wallpaper.id, error: String(error) },
          "error",
        );
        return false;
      }
    },
    [state.settings.qualityGuardMode],
  );

  const setWallpaper = useCallback(
    async (wallpaper: Wallpaper) => {
      if (!(await shouldApplyAfterQualityWarning(wallpaper))) {
        return;
      }
      const applied = await runWithStatus(
        `set-${wallpaper.id}`,
        () => invoke<Wallpaper>("set_wallpaper", { wallpaper }),
        "Wallpaper applied.",
      );
      if (applied) {
        dispatch({ type: "currentWallpaperChanged", wallpaper: applied });
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
      }
    },
    [refreshCacheStats, refreshLibrary, runWithStatus, shouldApplyAfterQualityWarning],
  );

  const setWallpaperWithLayout = useCallback(
    async (wallpaper: Wallpaper, layout: WallpaperLayoutPreference) => {
      if (!(await shouldApplyAfterQualityWarning(wallpaper))) {
        return;
      }
      const applied = await runWithStatus(
        `set-${wallpaper.id}`,
        () =>
          invoke<Wallpaper>("set_wallpaper_with_layout", {
            wallpaper,
            layout,
          }),
        "Wallpaper applied.",
      );
      if (applied) {
        dispatch({ type: "currentWallpaperChanged", wallpaper: applied });
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
      }
    },
    [refreshCacheStats, refreshLibrary, runWithStatus, shouldApplyAfterQualityWarning],
  );

  const setLockScreenWallpaper = useCallback(
    async (wallpaper: Wallpaper) => {
      await runWithStatus(
        `lock-${wallpaper.id}`,
        () => invoke<void>("set_lock_screen_wallpaper", { wallpaper }),
        "Lock-screen wallpaper updated.",
      );
    },
    [runWithStatus],
  );

  const assessWallpaperQuality = useCallback(
    async (wallpaper: Wallpaper) =>
      runWithStatus("quality", () =>
        invoke<WallpaperQualityReport>("assess_wallpaper_quality", {
          wallpaper,
        }),
      ),
    [runWithStatus],
  );

  const saveFavorite = useCallback(
    async (wallpaper: Wallpaper) => {
      const isSaved = state.library.favorites.some(
        (favorite) => favorite.id === wallpaper.id,
      );
      const favorite = !isSaved;
      const nextLibrary = await runWithStatus(
        `favorite-${wallpaper.id}`,
        () => invoke<Library>("set_favorite", { wallpaper, favorite }),
        favorite ? "Saved to favorites." : "Removed from favorites.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
      }
    },
    [runWithStatus, state.library.favorites],
  );

  const createPlaylist = useCallback(
    async (name: string) => {
      const nextLibrary = await runWithStatus(
        "playlist",
        () => invoke<Library>("create_playlist", { name }),
        "Playlist created.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
      }
    },
    [runWithStatus],
  );

  const deletePlaylist = useCallback(
    async (playlistId: string) => {
      const nextLibrary = await runWithStatus(
        `playlist-${playlistId}`,
        () => invoke<Library>("delete_playlist", { playlistId }),
        "Playlist deleted.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
      }
    },
    [runWithStatus],
  );

  const addWallpaperToPlaylist = useCallback(
    async (playlistId: string, wallpaper: Wallpaper) => {
      const nextLibrary = await runWithStatus(
        `playlist-${wallpaper.id}`,
        () =>
          invoke<Library>("add_wallpaper_to_playlist", {
            playlistId,
            wallpaper,
          }),
        "Added to playlist.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
      }
    },
    [runWithStatus],
  );

  const removeWallpaperFromPlaylist = useCallback(
    async (playlistId: string, wallpaperId: string) => {
      const nextLibrary = await runWithStatus(
        `playlist-${wallpaperId}`,
        () =>
          invoke<Library>("remove_wallpaper_from_playlist", {
            playlistId,
            wallpaperId,
          }),
        "Removed from playlist.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
      }
    },
    [runWithStatus],
  );

  const importLocalFolder = useCallback(
    async (folderPath: string) => {
      const result = await runWithStatus(
        "import-folder",
        () => invoke<ImportResult>("import_local_folder", { folderPath }),
        "Local folder imported.",
      );
      if (result) {
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
      }
      return result;
    },
    [refreshCacheStats, refreshLibrary, runWithStatus],
  );

  const exportBackup = useCallback(
    async (targetPath: string) =>
      runWithStatus(
        "backup",
        () => invoke<string>("export_backup", { targetPath }),
        "Backup exported.",
      ),
    [runWithStatus],
  );

  const importBackup = useCallback(
    async (sourcePath: string) => {
      const nextLibrary = await runWithStatus(
        "backup",
        () => invoke<Library>("import_backup", { sourcePath }),
        "Backup imported.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
        const loaded = await invoke<AppSettings>("get_settings");
        dispatch({ type: "settingsLoaded", settings: loaded });
        await refreshCacheStats();
      }
    },
    [refreshCacheStats, runWithStatus],
  );

  const runSupabaseAction = useCallback(
    async <T,>(
      label: string,
      action: () => Promise<T>,
      statusFromResult: (result: T) => SupabaseSyncStatus,
      applyResult?: (result: T) => Promise<void> | void,
    ): Promise<SupabaseSyncStatus> => {
      dispatch({ type: "busyChanged", busy: label });
      dispatch({ type: "noticeChanged", notice: "" });
      void logAppAction("sync.action.start", "Sync action started.", {
        label,
      });
      if (!isTauriRuntime()) {
        const status = failedSyncStatus(
          "Open the desktop app to test Supabase sync.",
        );
        dispatch({ type: "noticeChanged", notice: status.message });
        void logAppAction(
          "sync.action.unavailable",
          "Sync action skipped outside desktop runtime.",
          { label },
          "warn",
        );
        dispatch({ type: "busyChanged", busy: null });
        return status;
      }

      try {
        const result = await action();
        await applyResult?.(result);
        const status = statusFromResult(result);
        dispatch({ type: "noticeChanged", notice: syncNotice(status) });
        void logAppAction("sync.action.success", "Sync action completed.", {
          label,
          connected: status.connected,
          message: status.message,
        });
        return status;
      } catch (error) {
        const status = failedSyncStatus(String(error));
        dispatch({ type: "noticeChanged", notice: status.message });
        void logAppAction(
          "sync.action.error",
          "Sync action failed.",
          { label, error: status.message },
          "error",
        );
        return status;
      } finally {
        dispatch({ type: "busyChanged", busy: null });
      }
    },
    [],
  );

  const testSupabaseSync = useCallback(
    async (authContext?: SyncAuthContext | null) =>
      runSupabaseAction(
        "supabase-test",
        () =>
          invoke<SupabaseSyncStatus>("test_supabase_sync", {
            authContext: authContext ?? null,
          }),
        (status) => status,
      ),
    [runSupabaseAction],
  );

  const pushSupabaseSync = useCallback(
    async (authContext?: SyncAuthContext | null) =>
      runSupabaseAction(
        "supabase-push",
        () =>
          invoke<SupabaseSyncStatus>("push_supabase_sync", {
            authContext: authContext ?? null,
          }),
        (status) => status,
      ),
    [runSupabaseAction],
  );

  const pullSupabaseSync = useCallback(
    async (authContext?: SyncAuthContext | null) =>
      runSupabaseAction(
        "supabase-pull",
        () =>
          invoke<SupabaseSyncApplyResult>("pull_supabase_sync", {
            authContext: authContext ?? null,
          }),
        (result) => result.status,
        async (result) => {
          dispatch({ type: "settingsLoaded", settings: result.settings });
          dispatch({ type: "libraryLoaded", library: result.library });
          await refreshCacheStats();
        },
      ),
    [refreshCacheStats, runSupabaseAction],
  );

  const runAutoCleanup = useCallback(async () => {
    const stats = await runWithStatus(
      "auto-clean",
      () => invoke<CacheStats>("run_auto_cleanup"),
      "Auto-clean complete.",
    );
    if (stats) {
      dispatch({ type: "cacheStatsLoaded", cacheStats: stats });
      await refreshLibrary();
    }
  }, [refreshLibrary, runWithStatus]);

  const applyRandomWallpaper = useCallback(async () => {
    const wallpaper = await runWithStatus(
      "random",
      () => invoke<Wallpaper>("apply_random_wallpaper"),
      "Random wallpaper applied.",
    );
    if (wallpaper) {
      dispatch({ type: "currentWallpaperChanged", wallpaper });
      await Promise.all([refreshLibrary(), refreshCacheStats()]);
    }
  }, [refreshCacheStats, refreshLibrary, runWithStatus]);

  const applyNextWallpaper = useCallback(async () => {
    const wallpaper = await runWithStatus(
      "next",
      () => invoke<Wallpaper>("apply_next_wallpaper"),
      "Next wallpaper applied.",
    );
    if (wallpaper) {
      dispatch({ type: "currentWallpaperChanged", wallpaper });
      await Promise.all([refreshLibrary(), refreshCacheStats()]);
    }
  }, [refreshCacheStats, refreshLibrary, runWithStatus]);

  const toggleAutoChangePause = useCallback(
    async () =>
      runWithStatus(
        "pause",
        () => invoke<boolean>("toggle_auto_change_pause"),
        "Auto-change pause toggled.",
      ),
    [runWithStatus],
  );

  const applyMood = useCallback(
    async (nextMood: Mood) => {
      const nextQuery = pickRandomMoodQuery(moodQueries[nextMood]);
      void logAppAction("mood.apply", "Mood applied.", {
        mood: nextMood,
        query: nextQuery,
      });
      dispatch({ type: "moodChanged", mood: nextMood });
      dispatch({ type: "queryChanged", query: nextQuery });
      await searchWallpapers(1, nextQuery, state.source, nextMood);
    },
    [searchWallpapers, state.source],
  );

  const applyTopic = useCallback(
    async (nextQuery: string) => {
      void logAppAction("topic.apply", "Topic applied.", {
        query: nextQuery,
      });
      dispatch({ type: "queryChanged", query: nextQuery });
      await searchWallpapers(1, nextQuery);
    },
    [searchWallpapers],
  );

  const applyNextFromMood = useCallback(async () => {
    const nextQuery = pickRandomMoodQuery(moodQueries[state.mood]);
    void logAppAction("mood.next", "Next wallpaper from mood requested.", {
      mood: state.mood,
      query: nextQuery,
      source: state.source,
    });
    const wallpapers = await runWithStatus("next", () =>
      invoke<Wallpaper[]>("search_wallpapers", {
        query: nextQuery,
        page: 1,
        source: state.source,
      }),
    );
    const wallpaper = wallpapers
      ? pickRandomWallpaper(
          wallpapers.map((item) => ({ ...item, mood: state.mood })),
        )
      : null;

    if (wallpaper) {
      await setWallpaper(wallpaper);
    } else if (wallpapers) {
      dispatch({
        type: "noticeChanged",
        notice: "No wallpapers were returned for this mood.",
      });
    }
  }, [runWithStatus, setWallpaper, state.mood, state.source]);

  const clearWallpaperCache = useCallback(async () => {
    const stats = await runWithStatus(
      "clear-cache",
      () => invoke<CacheStats>("clear_cache"),
      "Cache cleared.",
    );
    if (stats) {
      dispatch({ type: "cacheStatsLoaded", cacheStats: stats });
      await refreshLibrary();
    }
  }, [refreshLibrary, runWithStatus]);

  const clearLibrary = useCallback(async () => {
    const nextLibrary = await runWithStatus(
      "clear-library",
      () => invoke<Library>("clear_library"),
      "Library cleared.",
    );
    if (nextLibrary) {
      dispatch({ type: "libraryLoaded", library: nextLibrary });
      await refreshCacheStats();
    }
  }, [refreshCacheStats, runWithStatus]);

  const deleteWallpaper = useCallback(
    async (wallpaper: Wallpaper) => {
      const nextLibrary = await runWithStatus(
        `delete-${wallpaper.id}`,
        () => invoke<Library>("delete_wallpaper", { id: wallpaper.id }),
        "Wallpaper deleted.",
      );
      if (nextLibrary) {
        dispatch({ type: "libraryLoaded", library: nextLibrary });
        await refreshCacheStats();
      }
    },
    [refreshCacheStats, runWithStatus],
  );

  const hasAnyKey = useMemo(
    () =>
      state.settings.apiKeys.pexels.trim().length > 0 ||
      state.settings.apiKeys.unsplash.trim().length > 0 ||
      state.settings.apiKeys.pixabay.trim().length > 0 ||
      state.settings.apiKeys.wallhaven.trim().length > 0 ||
      state.settings.apiKeys.deviantart.trim().length > 0,
    [
      state.settings.apiKeys.deviantart,
      state.settings.apiKeys.pexels,
      state.settings.apiKeys.pixabay,
      state.settings.apiKeys.unsplash,
      state.settings.apiKeys.wallhaven,
    ],
  );

  const favoriteIds = useMemo(
    () => new Set(state.library.favorites.map((wallpaper) => wallpaper.id)),
    [state.library.favorites],
  );

  const actions = useMemo<AppActions>(
    () => ({
      setActiveView: (view) => {
        void logAppAction("navigation.view.change", "Active view changed.", {
          from: state.activeView,
          to: view,
        });
        dispatch({ type: "activeViewChanged", activeView: view });
      },
      setQuery: (query) => {
        void logAppAction("search.query.change", "Search query changed.", {
          query,
        });
        dispatch({ type: "queryChanged", query });
      },
      searchWallpapers,
      changeSource,
      setWallpaper,
      setWallpaperWithLayout,
      setLockScreenWallpaper,
      assessWallpaperQuality,
      saveFavorite,
      createPlaylist,
      deletePlaylist,
      addWallpaperToPlaylist,
      removeWallpaperFromPlaylist,
      importLocalFolder,
      exportBackup,
      importBackup,
      testSupabaseSync,
      pushSupabaseSync,
      pullSupabaseSync,
      runAutoCleanup,
      applyNextWallpaper,
      toggleAutoChangePause,
      applyRandomWallpaper,
      applyMood,
      applyTopic,
      applyNextFromMood,
      clearWallpaperCache,
      clearLibrary,
      deleteWallpaper,
      saveSettings,
    }),
    [
      addWallpaperToPlaylist,
      applyNextWallpaper,
      applyMood,
      applyNextFromMood,
      applyRandomWallpaper,
      applyTopic,
      assessWallpaperQuality,
      changeSource,
      clearLibrary,
      clearWallpaperCache,
      createPlaylist,
      deleteWallpaper,
      deletePlaylist,
      exportBackup,
      importBackup,
      importLocalFolder,
      pullSupabaseSync,
      pushSupabaseSync,
      removeWallpaperFromPlaylist,
      runAutoCleanup,
      saveFavorite,
      saveSettings,
      searchWallpapers,
      setLockScreenWallpaper,
      setWallpaper,
      setWallpaperWithLayout,
      testSupabaseSync,
      toggleAutoChangePause,
      state.activeView,
    ],
  );

  const value = useMemo<AppStateValue>(
    () => ({
      ...state,
      hasAnyKey,
      favoriteIds,
      actions,
    }),
    [actions, favoriteIds, hasAnyKey, state],
  );

  return (
    <AppStateContext.Provider value={value}>
      {children}
    </AppStateContext.Provider>
  );
}

function syncNotice(status: SupabaseSyncStatus): string {
  if (!status.connected) {
    return status.message;
  }
  return status.updatedAt
    ? `${status.message} Last cloud update: ${status.updatedAt}.`
    : status.message;
}

function failedSyncStatus(message: string): SupabaseSyncStatus {
  return {
    connected: false,
    message,
    updatedAt: null,
  };
}

export function useAppState(): AppStateValue {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error("useAppState must be used within AppStateProvider");
  }

  return context;
}
