import { describe, expect, it } from "vitest";
import { buildProviderQuery, filterWallpapers } from "./searchFilters";
import { SearchFilters, Wallpaper } from "./types";

function wallpaper(id: string, width: number, height: number): Wallpaper {
  return {
    id,
    source: "pexels",
    thumbUrl: "",
    fullUrl: "",
    photographer: "Example",
    width,
    height,
    queryUsed: null,
    mood: null,
    localPath: null,
    isFavorite: false,
  };
}

const baseFilters: SearchFilters = {
  orientation: "any",
  minWidth: 0,
  minHeight: 0,
  color: "",
};

describe("buildProviderQuery", () => {
  it("adds a color hint only when one is configured", () => {
    expect(buildProviderQuery("city", baseFilters)).toBe("city");
    expect(buildProviderQuery("city", { ...baseFilters, color: " blue " })).toBe(
      "city blue",
    );
  });
});

describe("filterWallpapers", () => {
  it("filters by minimum size and orientation", () => {
    const wallpapers = [
      wallpaper("landscape", 3840, 2160),
      wallpaper("portrait", 1200, 1800),
      wallpaper("small", 1280, 720),
    ];

    expect(
      filterWallpapers(wallpapers, {
        ...baseFilters,
        orientation: "landscape",
        minWidth: 1920,
        minHeight: 1080,
      }).map((item) => item.id),
    ).toEqual(["landscape"]);
  });

  it("keeps unknown dimensions visible instead of hiding possibly valid results", () => {
    expect(
      filterWallpapers([wallpaper("unknown", 0, 0)], {
        ...baseFilters,
        orientation: "landscape",
        minWidth: 4000,
      }).map((item) => item.id),
    ).toEqual(["unknown"]);
  });
});
