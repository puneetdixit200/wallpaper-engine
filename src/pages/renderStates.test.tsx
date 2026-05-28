import { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import {
  AppStateContext,
  AppStateValue,
  initialAppState,
} from "../appState";
import { WallCard } from "../components/WallCard";
import { LibraryPage } from "./Library";
import { SearchPage } from "./Search";
import { SettingsPage } from "./Settings";
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
  const favoriteIds = new Set(
    (overrides.library ?? initialAppState.library).favorites.map(
      (item) => item.id,
    ),
  );

  return {
    ...initialAppState,
    hasAnyKey: false,
    favoriteIds,
    actions: {
      setActiveView: noop,
      setQuery: noop,
      searchWallpapers: asyncNoop,
      changeSource: asyncNoop,
      setWallpaper: asyncNoop,
      saveFavorite: asyncNoop,
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
        library: { favorites: [saved], downloaded: [] },
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
        library: { favorites: [], downloaded: [] },
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
        library: { favorites: [saved], downloaded: [] },
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
});
