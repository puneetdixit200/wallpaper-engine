import { AppSettings, Wallpaper, WallpaperLayoutPreference } from "./types";

export function wallpaperQualityWarnings(
  wallpaper: Wallpaper,
  settings: AppSettings,
): string[] {
  if (settings.qualityGuardMode === "off") {
    return [];
  }

  const warnings: string[] = [];
  if (wallpaper.width === 0 || wallpaper.height === 0) {
    warnings.push("Resolution is unknown.");
    return warnings;
  }

  if (wallpaper.width < settings.qualityMinWidth) {
    warnings.push(`Width is below ${settings.qualityMinWidth}px.`);
  }

  if (wallpaper.height < settings.qualityMinHeight) {
    warnings.push(`Height is below ${settings.qualityMinHeight}px.`);
  }

  if (!settings.allowPortraitWallpapers && wallpaper.height > wallpaper.width) {
    warnings.push("Portrait image may look cropped on landscape displays.");
  }

  if ((wallpaper.width / wallpaper.height) * 100 < 120) {
    warnings.push("Aspect ratio is narrow for most desktops.");
  }

  return warnings;
}

export function objectFitForLayout(
  layout: WallpaperLayoutPreference,
): "cover" | "contain" | "fill" | "none" {
  switch (layout) {
    case "fill":
    case "span":
      return "cover";
    case "fit":
    case "center":
      return "contain";
    case "stretch":
      return "fill";
    case "tile":
      return "none";
  }
}
