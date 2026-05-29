export type ViewName = "home" | "search" | "library" | "settings";
export type ApiSource =
  | "all"
  | "pexels"
  | "unsplash"
  | "pixabay"
  | "wallhaven"
  | "picsum"
  | "deviantArt"
  | "artStation";
export type ResolutionPreference = "auto" | "fullHd" | "fourK";
export type ThemePreference = "system" | "light" | "dark";
export type WallpaperLayoutPreference =
  | "fill"
  | "fit"
  | "stretch"
  | "tile"
  | "center"
  | "span";
export type QualityGuardMode = "off" | "warn" | "skip";
export type SearchOrientationFilter = "any" | "landscape" | "portrait" | "square";

export type Mood =
  | "dark"
  | "nature"
  | "city"
  | "minimal"
  | "coding"
  | "calm"
  | "anime"
  | "cyberpunk"
  | "space"
  | "gaming"
  | "fantasy"
  | "cars";

export interface ApiKeys {
  pexels: string;
  unsplash: string;
  pixabay: string;
  wallhaven: string;
  deviantart: string;
}

export interface AppSettings {
  apiKeys: ApiKeys;
  autoChangeMinutes: number;
  resolution: ResolutionPreference;
  cacheLimitMb: number;
  allowNsfwWallhaven: boolean;
  theme: ThemePreference;
  wallpaperLayout: WallpaperLayoutPreference;
  runInBackground: boolean;
  launchAtStartup: boolean;
  applyToLockScreen: boolean;
  globalHotkeysEnabled: boolean;
  qualityGuardMode: QualityGuardMode;
  qualityMinWidth: number;
  qualityMinHeight: number;
  allowPortraitWallpapers: boolean;
  searchFilters: SearchFilters;
  activePlaylistId: string | null;
  hotkeys: HotkeySettings;
  autoCleanDays: number;
  autoCleanKeepFavorites: boolean;
}

export interface Wallpaper {
  id: string;
  source: string;
  thumbUrl: string;
  fullUrl: string;
  photographer: string;
  width: number;
  height: number;
  queryUsed?: string | null;
  mood?: Mood | null;
  localPath?: string | null;
  isFavorite: boolean;
}

export interface WallpaperPlaylist {
  id: string;
  name: string;
  wallpapers: Wallpaper[];
}

export interface Library {
  favorites: Wallpaper[];
  downloaded: Wallpaper[];
  playlists: WallpaperPlaylist[];
}

export interface CacheStats {
  bytes: number;
  files: number;
}

export interface ImportResult {
  imported: number;
  skipped: number;
}

export interface WallpaperQualityReport {
  ok: boolean;
  warnings: string[];
}

export interface SearchFilters {
  orientation: SearchOrientationFilter;
  minWidth: number;
  minHeight: number;
  color: string;
}

export interface HotkeySettings {
  nextWallpaper: string;
  pauseRotation: string;
  favoriteCurrent: string;
}

export const defaultSettings: AppSettings = {
  apiKeys: {
    pexels: "",
    unsplash: "",
    pixabay: "",
    wallhaven: "",
    deviantart: "",
  },
  autoChangeMinutes: 0,
  resolution: "auto",
  cacheLimitMb: 1024,
  allowNsfwWallhaven: false,
  theme: "system",
  wallpaperLayout: "fit",
  runInBackground: false,
  launchAtStartup: false,
  applyToLockScreen: false,
  globalHotkeysEnabled: true,
  qualityGuardMode: "warn",
  qualityMinWidth: 1920,
  qualityMinHeight: 1080,
  allowPortraitWallpapers: false,
  searchFilters: {
    orientation: "any",
    minWidth: 0,
    minHeight: 0,
    color: "",
  },
  activePlaylistId: null,
  hotkeys: {
    nextWallpaper: "CommandOrControl+Alt+N",
    pauseRotation: "CommandOrControl+Alt+P",
    favoriteCurrent: "CommandOrControl+Alt+F",
  },
  autoCleanDays: 0,
  autoCleanKeepFavorites: true,
};

export const moodQueries: Record<Mood, string[]> = {
  dark: ["dark minimal", "black abstract", "dark aesthetic"],
  nature: ["forest", "mountains", "ocean sunset", "green nature"],
  city: ["city night", "urban architecture", "skyline"],
  minimal: ["minimalist clean", "white simple", "geometric minimal"],
  coding: ["dark workspace", "terminal aesthetic", "neon dark"],
  calm: ["zen nature", "soft pastel", "peaceful landscape"],
  anime: ["anime landscape", "anime scenery", "painted landscape"],
  cyberpunk: ["cyberpunk city", "neon street", "futuristic skyline"],
  space: ["space nebula", "galaxy wallpaper", "astronaut landscape"],
  gaming: ["gaming setup", "neon gaming", "game landscape"],
  fantasy: ["fantasy castle", "dragon landscape", "magic forest"],
  cars: ["supercar night", "jdm car", "sports car wallpaper"],
};

export const trendingTopics = [
  { label: "4K anime city", query: "4k anime city" },
  { label: "cyberpunk neon", query: "cyberpunk neon city" },
  { label: "space nebula", query: "space nebula 4k" },
  { label: "supercar night", query: "supercar night wallpaper" },
  { label: "cozy rain", query: "cozy rainy window" },
  { label: "fantasy world", query: "fantasy landscape 4k" },
  { label: "minimal dark", query: "minimal dark wallpaper" },
  { label: "ultrawide nature", query: "ultrawide nature wallpaper" },
  { label: "gaming setup", query: "gaming setup neon" },
  { label: "cinematic mountains", query: "cinematic mountain landscape" },
];
