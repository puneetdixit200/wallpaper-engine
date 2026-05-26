import { describe, expect, it } from "vitest";
import { pickRandomMoodQuery, pickRandomWallpaper } from "./wallpaperSelection";
import { Wallpaper } from "./types";

function wallpaper(id: string): Wallpaper {
  return {
    id,
    source: "test",
    thumbUrl: "",
    fullUrl: "",
    photographer: "Test",
    width: 1920,
    height: 1080,
    isFavorite: false,
  };
}

describe("pickRandomWallpaper", () => {
  it("uses the random value instead of always selecting the first result", () => {
    const wallpapers = [wallpaper("first"), wallpaper("middle"), wallpaper("last")];

    expect(pickRandomWallpaper(wallpapers, () => 0.99)?.id).toBe("last");
    expect(pickRandomWallpaper(wallpapers, () => 0)?.id).toBe("first");
    expect(pickRandomWallpaper([], () => 0.99)).toBeNull();
  });
});

describe("pickRandomMoodQuery", () => {
  it("uses all configured mood query options", () => {
    const queries = ["dark minimal", "black abstract", "dark aesthetic"];

    expect(pickRandomMoodQuery(queries, () => 0.8)).toBe("dark aesthetic");
    expect(pickRandomMoodQuery([], () => 0.8)).toBe("");
  });
});
