import { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  AppStateContext,
  AppStateValue,
  initialAppState,
} from "../appState";
import { WallCard } from "../components/WallCard";
import { ControlsPage } from "./Controls";
import { LibraryPage } from "./Library";
import { SearchPage } from "./Search";
import { SettingsPage } from "./Settings";
import { SyncPage } from "./Sync";
import { Wallpaper } from "../types";

function wallpaper(id: string, favorite = false): Wallpaper {
  return {
    id,
    source: "pexels",
    thumbUrl: "",
    fullUrl: "",
    photographer: "Example",
    width: 1920,
    height: 1080,
    queryUsed: "forest",
    mood: null,
    localPath: null,
    isFavorite: favorite,
  };
}

const noop = () => undefined;
const asyncNoop = async () => undefined;

function appValue(overrides: Partial<AppStateValue> = {}): AppStateValue {
  const library = {
    ...initialAppState.library,
    ...overrides.library,
  };
  const favoriteIds = new Set(library.favorites.map((item) => item.id));

  return {
    ...initialAppState,
    library,
    hasAnyKey: false,
    favoriteIds,
    actions: {
      setActiveView: noop,
      setQuery: noop,
      searchWallpapers: asyncNoop,
      changeSource: asyncNoop,
      setWallpaper: asyncNoop,
      setWallpaperWithLayout: asyncNoop,
      setLockScreenWallpaper: asyncNoop,
      assessWallpaperQuality: async () => null,
      saveFavorite: asyncNoop,
      createPlaylist: asyncNoop,
      deletePlaylist: asyncNoop,
      addWallpaperToPlaylist: asyncNoop,
      removeWallpaperFromPlaylist: asyncNoop,
      importLocalFolder: async () => null,
      exportBackup: async () => null,
      importBackup: asyncNoop,
      testSupabaseSync: async () => null,
      pushSupabaseSync: async () => null,
      pullSupabaseSync: async () => null,
      runAutoCleanup: asyncNoop,
      applyNextWallpaper: asyncNoop,
      toggleAutoChangePause: async () => null,
      applyRandomWallpaper: asyncNoop,
      applyMood: asyncNoop,
      applyTopic: asyncNoop,
      applyNextFromMood: asyncNoop,
      clearWallpaperCache: asyncNoop,
      clearLibrary: asyncNoop,
      deleteWallpaper: asyncNoop,
      saveSettings: asyncNoop,
    },
    ...overrides,
  };
}

function renderWithState(children: ReactNode, value: AppStateValue): string {
  return renderToStaticMarkup(
    <AppStateContext.Provider value={value}>{children}</AppStateContext.Provider>,
  );
}

describe("page render states", () => {
  it("shows an empty search state before results are loaded", () => {
    const html = renderWithState(<SearchPage />, appValue());

    expect(html).toContain("No results yet");
  });

  it("shows search skeleton cards while a search is loading", () => {
    const html = renderWithState(
      <SearchPage />,
      appValue({ busy: "search", results: [] }),
    );

    expect(html).toContain("wall-skeleton");
  });

  it("shows library empty states for each empty section", () => {
    const html = renderWithState(<LibraryPage />, appValue());

    expect(html).toContain("No favorites yet");
    expect(html).toContain("No downloads yet");
  });

  it("keeps clear library available when cached downloads exist without visible library items", () => {
    const html = renderWithState(
      <LibraryPage />,
      appValue({
        cacheStats: {
          bytes: 1024,
          files: 2,
        },
      }),
    );

    expect(html).toContain("Clear library");
    expect(html).not.toMatch(/<button[^>]*disabled[^>]*>Clear library/);
  });

  it("marks saved wallpapers with a filled favorite action", () => {
    const saved = wallpaper("saved", true);
    const html = renderWithState(
      <WallCard wallpaper={saved} />,
      appValue({
        library: { favorites: [saved], downloaded: [], playlists: [] },
        favoriteIds: new Set(["saved"]),
      }),
    );

    expect(html).toContain('aria-label="Saved favorite"');
    expect(html).toContain("icon-button saved");
  });

  it("does not render stale wallpaper favorite flags as saved", () => {
    const stale = wallpaper("stale", true);
    const html = renderWithState(
      <WallCard wallpaper={stale} />,
      appValue({
        library: { favorites: [], downloaded: [], playlists: [] },
        favoriteIds: new Set(),
      }),
    );

    expect(html).toContain('aria-label="Save favorite"');
    expect(html).not.toContain("icon-button saved");
  });

  it("shows delete action only for library wallpaper cards", () => {
    const saved = wallpaper("saved", true);
    const libraryHtml = renderWithState(
      <WallCard wallpaper={saved} canDelete />,
      appValue({
        library: { favorites: [saved], downloaded: [], playlists: [] },
        favoriteIds: new Set(["saved"]),
      }),
    );
    const searchHtml = renderWithState(<WallCard wallpaper={saved} />, appValue());

    expect(libraryHtml).toContain('aria-label="Delete wallpaper"');
    expect(searchHtml).not.toContain('aria-label="Delete wallpaper"');
  });

  it("surfaces automatic change, after-close, and startup controls in settings", () => {
    const html = renderWithState(<SettingsPage />, appValue());

    expect(html).toContain("Automatic wallpaper changes");
    expect(html).toContain("Change every");
    expect(html).toContain("Keep changing after close");
    expect(html).toContain("Start at login");
  });

  it("shows playlist, import, and backup tools in the library", () => {
    const html = renderWithState(<LibraryPage />, appValue());

    expect(html).toContain("Playlists");
    expect(html).toContain("Local folder path");
    expect(html).toContain("Export backup path");
  });

  it("keeps quality guard and hotkey controls out of settings", () => {
    const html = renderWithState(<SettingsPage />, appValue());

    expect(html).toContain("Run auto-clean");
    expect(html).not.toContain("Quality guard");
    expect(html).not.toContain("Enable global hotkeys");
  });

  it("shows quality guard and hotkey controls in controls", () => {
    const html = renderWithState(<ControlsPage />, appValue());

    expect(html).toContain("Quality guard");
    expect(html).toContain("Enable global hotkeys");
    expect(html).toContain("Next wallpaper hotkey");
    expect(html).toContain("Pause timer hotkey");
    expect(html).toContain("Favorite current hotkey");
    expect(html).toContain("Save controls");
  });

  it("shows the GitHub credit in settings", () => {
    const html = renderWithState(<SettingsPage />, appValue());

    expect(html).toContain("Made with");
    expect(html).toContain("https://github.com/puneetdixit200");
    expect(html).toContain("puneetdixit");
  });

  it("shows Supabase sync configuration and actions", () => {
    const html = renderWithState(<SyncPage />, appValue());

    expect(html).toContain("Supabase cloud sync");
    expect(html).toContain("Project URL");
    expect(html).toContain("Anon key");
    expect(html).toContain("Sync ID");
    expect(html).toContain("Use Clerk login for sync");
    expect(html).toContain("Clerk publishable key");
    expect(html).toContain("Clerk sign-in inactive");
    expect(html).toContain("Not connected");
    expect(html).toContain("Supabase sync is off.");
    expect(html).toContain("wallpaper_engine_sync");
    expect(html).toContain("Push");
    expect(html).toContain("Pull");
  });
});
