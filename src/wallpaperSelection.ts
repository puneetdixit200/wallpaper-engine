import { Wallpaper } from "./types";

type RandomSource = () => number;

function randomIndex(length: number, random: RandomSource): number {
  return Math.min(Math.floor(random() * length), length - 1);
}

export function pickRandomWallpaper(
  wallpapers: Wallpaper[],
  random: RandomSource = Math.random,
): Wallpaper | null {
  if (wallpapers.length === 0) {
    return null;
  }

  return wallpapers[randomIndex(wallpapers.length, random)];
}

export function pickRandomMoodQuery(
  queries: string[],
  random: RandomSource = Math.random,
): string {
  if (queries.length === 0) {
    return "";
  }

  return queries[randomIndex(queries.length, random)];
}
