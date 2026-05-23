import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Database,
  Heart,
  Home,
  Image,
  Search,
  Settings,
} from "lucide-react";
import { HomePage } from "./pages/Home";
import { SearchPage } from "./pages/Search";
import { LibraryPage } from "./pages/Library";
import { SettingsPage } from "./pages/Settings";
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
import "./App.css";

const navItems: Array<{ id: ViewName; label: string; icon: typeof Home }> = [
  { id: "home", label: "Home", icon: Home },
  { id: "search", label: "Search", icon: Search },
  { id: "library", label: "Library", icon: Image },
  { id: "settings", label: "Settings", icon: Settings },
];

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function App() {
  const [activeView, setActiveView] = useState<ViewName>("home");
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [currentWallpaper, setCurrentWallpaper] = useState<Wallpaper | null>(
    null,
  );
  const [library, setLibrary] = useState<Library>({
    favorites: [],
    downloaded: [],
  });
  const [cacheStats, setCacheStats] = useState<CacheStats>({
    bytes: 0,
    files: 0,
  });
  const [query, setQuery] = useState("nature");
  const [source, setSource] = useState<ApiSource>("both");
  const [mood, setMood] = useState<Mood>("nature");
  const [results, setResults] = useState<Wallpaper[]>([]);
  const [page, setPage] = useState(1);
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState("");

  const hasAnyKey = useMemo(
    () =>
      settings.apiKeys.pexels.trim().length > 0 ||
      settings.apiKeys.unsplash.trim().length > 0,
    [settings.apiKeys.pexels, settings.apiKeys.unsplash],
  );

  const refreshLibrary = useCallback(async () => {
    const nextLibrary = await invoke<Library>("list_library");
    setLibrary(nextLibrary);
  }, []);

  const refreshCacheStats = useCallback(async () => {
    const stats = await invoke<CacheStats>("cache_stats");
    setCacheStats(stats);
  }, []);

  useEffect(() => {
    async function boot() {
      if (!isTauriRuntime()) {
        return;
      }

      try {
        const loaded = await invoke<AppSettings>("get_settings");
        setSettings(loaded);
        await Promise.all([refreshLibrary(), refreshCacheStats()]);
      } catch (error) {
        setNotice(String(error));
      }
    }

    boot();
  }, [refreshCacheStats, refreshLibrary]);

  async function runWithStatus<T>(
    label: string,
    action: () => Promise<T>,
    done?: string,
  ): Promise<T | null> {
    setBusy(label);
    setNotice("");
    if (!isTauriRuntime()) {
      setNotice("Open the desktop app to use wallpaper actions.");
      setBusy(null);
      return null;
    }

    try {
      const value = await action();
      if (done) {
        setNotice(done);
      }
      return value;
    } catch (error) {
      setNotice(String(error));
      return null;
    } finally {
      setBusy(null);
    }
  }

  async function saveSettings(nextSettings: AppSettings) {
    const saved = await runWithStatus(
      "settings",
      () => invoke<AppSettings>("save_settings", { settings: nextSettings }),
      "Settings saved.",
    );
    if (saved) {
      setSettings(saved);
    }
  }

  async function searchWallpapers(nextPage = 1, nextQuery = query) {
    const wallpapers = await runWithStatus("search", () =>
      invoke<Wallpaper[]>("search_wallpapers", {
        query: nextQuery,
        page: nextPage,
        source,
      }),
    );
    if (!wallpapers) {
      return;
    }

    setPage(nextPage);
    setResults((existing) =>
      nextPage === 1 ? wallpapers : [...existing, ...wallpapers],
    );
    setActiveView("search");
  }

  async function setWallpaper(wallpaper: Wallpaper) {
    const applied = await runWithStatus(
      `set-${wallpaper.id}`,
      () => invoke<Wallpaper>("set_wallpaper", { wallpaper }),
      "Wallpaper applied.",
    );
    if (applied) {
      setCurrentWallpaper(applied);
      await Promise.all([refreshLibrary(), refreshCacheStats()]);
    }
  }

  async function saveFavorite(wallpaper: Wallpaper) {
    await runWithStatus(
      `favorite-${wallpaper.id}`,
      () => invoke("save_favorite", { wallpaper }),
      "Saved to favorites.",
    );
    await refreshLibrary();
  }

  async function applyRandomWallpaper() {
    const wallpaper = await runWithStatus(
      "random",
      () => invoke<Wallpaper>("apply_random_wallpaper"),
      "Random wallpaper applied.",
    );
    if (wallpaper) {
      setCurrentWallpaper(wallpaper);
      await Promise.all([refreshLibrary(), refreshCacheStats()]);
    }
  }

  async function applyMood(nextMood: Mood) {
    setMood(nextMood);
    const nextQuery = moodQueries[nextMood][0];
    setQuery(nextQuery);
    await searchWallpapers(1, nextQuery);
  }

  async function applyNextFromMood() {
    const nextQuery = moodQueries[mood][0];
    const wallpapers = await runWithStatus("next", () =>
      invoke<Wallpaper[]>("search_wallpapers", {
        query: nextQuery,
        page: 1,
        source,
      }),
    );
    const wallpaper = wallpapers?.[0];
    if (wallpaper) {
      await setWallpaper(wallpaper);
    }
  }

  async function clearWallpaperCache() {
    const stats = await runWithStatus(
      "clear-cache",
      () => invoke<CacheStats>("clear_cache"),
      "Cache cleared.",
    );
    if (stats) {
      setCacheStats(stats);
      await refreshLibrary();
    }
  }

  const content =
    activeView === "home" ? (
      <HomePage
        busy={busy}
        currentWallpaper={currentWallpaper}
        hasAnyKey={hasAnyKey}
        mood={mood}
        notice={notice}
        onMoodSelect={applyMood}
        onNext={applyNextFromMood}
        onRandom={applyRandomWallpaper}
        onSaveCurrent={() =>
          currentWallpaper ? saveFavorite(currentWallpaper) : undefined
        }
      />
    ) : activeView === "search" ? (
      <SearchPage
        busy={busy}
        page={page}
        query={query}
        results={results}
        source={source}
        onLoadMore={() => searchWallpapers(page + 1)}
        onQueryChange={setQuery}
        onSearch={() => searchWallpapers(1)}
        onSetWallpaper={setWallpaper}
        onSaveFavorite={saveFavorite}
        onSourceChange={setSource}
      />
    ) : activeView === "library" ? (
      <LibraryPage
        busy={busy}
        library={library}
        onSetWallpaper={setWallpaper}
        onSaveFavorite={saveFavorite}
      />
    ) : (
      <SettingsPage
        busy={busy}
        cacheStats={cacheStats}
        settings={settings}
        onClearCache={clearWallpaperCache}
        onSave={saveSettings}
      />
    );

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <Database size={18} aria-hidden="true" />
          </div>
          <div>
            <h1>Wallpaper Engine</h1>
            <p>Desktop wallpaper control</p>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                className={activeView === item.id ? "nav-item active" : "nav-item"}
                key={item.id}
                onClick={() => setActiveView(item.id)}
                type="button"
              >
                <Icon size={18} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-footer">
          <Heart size={16} aria-hidden="true" />
          <span>{library.favorites.length} saved</span>
        </div>
      </aside>

      <section className="content-shell">{content}</section>
    </main>
  );
}

export default App;
