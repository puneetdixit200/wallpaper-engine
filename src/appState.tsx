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
  ViewName,
  Wallpaper,
} from "./types";
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
  saveFavorite: (wallpaper: Wallpaper) => Promise<void>;
  applyRandomWallpaper: () => Promise<void>;
  applyMood: (mood: Mood) => Promise<void>;
  applyTopic: (query: string) => Promise<void>;
  applyNextFromMood: () => Promise<void>;
  clearWallpaperCache: () => Promise<void>;
  clearLibrary: () => Promise<void>;
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
        const loaded = await invoke<AppSettings>("get_settings");
        dispatch({ type: "settingsLoaded", settings: loaded });
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
      } catch (error) {
        dispatch({ type: "noticeChanged", notice: String(error) });
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
      if (!isTauriRuntime()) {
        dispatch({
          type: "noticeChanged",
          notice: "Open the desktop app to use wallpaper actions.",
        });
        dispatch({ type: "busyChanged", busy: null });
        return null;
      }

      try {
        const value = await action();
        if (done) {
          dispatch({ type: "noticeChanged", notice: done });
        }
        return value;
      } catch (error) {
        dispatch({ type: "noticeChanged", notice: String(error) });
        return null;
      } finally {
        dispatch({ type: "busyChanged", busy: null });
      }
    },
    [],
  );

  const saveSettings = useCallback(
    async (nextSettings: AppSettings) => {
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

  const setWallpaper = useCallback(
    async (wallpaper: Wallpaper) => {
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
    [refreshCacheStats, refreshLibrary, runWithStatus],
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

  const applyMood = useCallback(
    async (nextMood: Mood) => {
      const nextQuery = pickRandomMoodQuery(moodQueries[nextMood]);
      dispatch({ type: "moodChanged", mood: nextMood });
      dispatch({ type: "queryChanged", query: nextQuery });
      await searchWallpapers(1, nextQuery, state.source, nextMood);
    },
    [searchWallpapers, state.source],
  );

  const applyTopic = useCallback(
    async (nextQuery: string) => {
      dispatch({ type: "queryChanged", query: nextQuery });
      await searchWallpapers(1, nextQuery);
    },
    [searchWallpapers],
  );

  const applyNextFromMood = useCallback(async () => {
    const nextQuery = pickRandomMoodQuery(moodQueries[state.mood]);
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
      setActiveView: (view) =>
        dispatch({ type: "activeViewChanged", activeView: view }),
      setQuery: (query) => dispatch({ type: "queryChanged", query }),
      searchWallpapers,
      changeSource,
      setWallpaper,
      saveFavorite,
      applyRandomWallpaper,
      applyMood,
      applyTopic,
      applyNextFromMood,
      clearWallpaperCache,
      clearLibrary,
      saveSettings,
    }),
    [
      applyMood,
      applyNextFromMood,
      applyRandomWallpaper,
      applyTopic,
      changeSource,
      clearLibrary,
      clearWallpaperCache,
      saveFavorite,
      saveSettings,
      searchWallpapers,
      setWallpaper,
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

export function useAppState(): AppStateValue {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error("useAppState must be used within AppStateProvider");
  }

  return context;
}
