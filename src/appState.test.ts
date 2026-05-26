import { describe, expect, it } from "vitest";
import { appReducer, initialAppState } from "./appState";
import { Wallpaper } from "./types";

function wallpaper(id: string): Wallpaper {
  return {
    id,
    source: "pexels",
    thumbUrl: `https://example.com/${id}-thumb.jpg`,
    fullUrl: `https://example.com/${id}.jpg`,
    photographer: "Example",
    width: 3840,
    height: 2160,
    queryUsed: "forest",
    mood: null,
    localPath: null,
    isFavorite: false,
  };
}

describe("appReducer", () => {
  it("replaces first-page search results and appends later pages", () => {
    const firstPage = appReducer(initialAppState, {
      type: "searchLoaded",
      page: 1,
      wallpapers: [wallpaper("one")],
    });

    expect(firstPage.activeView).toBe("search");
    expect(firstPage.page).toBe(1);
    expect(firstPage.results.map((item) => item.id)).toEqual(["one"]);

    const secondPage = appReducer(firstPage, {
      type: "searchLoaded",
      page: 2,
      wallpapers: [wallpaper("two")],
    });

    expect(secondPage.page).toBe(2);
    expect(secondPage.results.map((item) => item.id)).toEqual(["one", "two"]);
  });
});
