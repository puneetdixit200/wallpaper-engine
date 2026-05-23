export type ViewName = "home" | "search" | "library" | "settings";
export type ApiSource =
  | "all"
  | "pexels"
  | "unsplash"
  | "pixabay"
  | "wallhaven"
  | "picsum"
  | "deviantArt"
  | "artStation"
  | "both";
export type ResolutionPreference = "auto" | "fullHd" | "fourK";

export type Mood =
  | "dark"
  | "nature"
  | "city"
  | "minimal"
  | "coding"
  | "calm"
  | "anime";

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
  localPath?: string | null;
  isFavorite: boolean;
}

export interface Library {
  favorites: Wallpaper[];
  downloaded: Wallpaper[];
}

export interface CacheStats {
  bytes: number;
  files: number;
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
};

export const moodQueries: Record<Mood, string[]> = {
  dark: ["dark minimal", "black abstract", "dark aesthetic"],
  nature: ["forest", "mountains", "ocean sunset", "green nature"],
  city: ["city night", "urban architecture", "skyline"],
  minimal: ["minimalist clean", "white simple", "geometric minimal"],
  coding: ["dark workspace", "terminal aesthetic", "neon dark"],
  calm: ["zen nature", "soft pastel", "peaceful landscape"],
  anime: ["anime landscape", "anime scenery", "painted landscape"],
};
