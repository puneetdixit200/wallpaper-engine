import { SearchFilters, Wallpaper } from "./types";

export function buildProviderQuery(query: string, filters: SearchFilters): string {
  const color = filters.color.trim();
  if (!color) {
    return query;
  }

  const normalizedQuery = query.trim();
  return normalizedQuery ? `${normalizedQuery} ${color}` : color;
}

export function filterWallpapers(
  wallpapers: Wallpaper[],
  filters: SearchFilters,
): Wallpaper[] {
  return wallpapers.filter((wallpaper) => matchesFilters(wallpaper, filters));
}

export function matchesFilters(
  wallpaper: Wallpaper,
  filters: SearchFilters,
): boolean {
  if (filters.minWidth > 0 && wallpaper.width > 0 && wallpaper.width < filters.minWidth) {
    return false;
  }

  if (
    filters.minHeight > 0 &&
    wallpaper.height > 0 &&
    wallpaper.height < filters.minHeight
  ) {
    return false;
  }

  if (filters.orientation === "any" || wallpaper.width === 0 || wallpaper.height === 0) {
    return true;
  }

  if (filters.orientation === "landscape") {
    return wallpaper.width > wallpaper.height;
  }

  if (filters.orientation === "portrait") {
    return wallpaper.height > wallpaper.width;
  }

  const spread = Math.abs(wallpaper.width - wallpaper.height);
  return spread <= Math.max(wallpaper.width, wallpaper.height) * 0.08;
}
