import { describe, expect, it } from "vitest";
import { defaultSettings, Wallpaper } from "./types";
import {
  objectFitForLayout,
  wallpaperQualityWarnings,
} from "./wallpaperQuality";

function wallpaper(width: number, height: number): Wallpaper {
  return {
    id: `${width}x${height}`,
    source: "local",
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

describe("wallpaperQualityWarnings", () => {
  it("warns for low-resolution and portrait wallpapers", () => {
    const warnings = wallpaperQualityWarnings(wallpaper(900, 1600), {
      ...defaultSettings,
      qualityGuardMode: "warn",
    });

    expect(warnings.join(" ")).toContain("Width is below");
    expect(warnings.join(" ")).toContain("Portrait image");
  });

  it("does not warn when the guard is disabled", () => {
    expect(
      wallpaperQualityWarnings(wallpaper(900, 600), {
        ...defaultSettings,
        qualityGuardMode: "off",
      }),
    ).toEqual([]);
  });
});

describe("objectFitForLayout", () => {
  it("maps layout modes to preview object-fit values", () => {
    expect(objectFitForLayout("fill")).toBe("cover");
    expect(objectFitForLayout("fit")).toBe("contain");
    expect(objectFitForLayout("stretch")).toBe("fill");
    expect(objectFitForLayout("tile")).toBe("none");
  });
});
