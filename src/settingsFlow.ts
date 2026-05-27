import { AppSettings } from "./types";

export const backgroundPermissionMessage =
  "Allow Wallpaper Engine to run in the tray and start at login so wallpapers keep changing after you close the window?";

export function parseAutoChangeMinutes(value: string): number {
  const minutes = Number(value);
  if (!Number.isFinite(minutes)) {
    return 0;
  }

  return Math.min(Math.max(Math.trunc(minutes), 0), 1440);
}

export function parseCacheLimitMb(value: string): number {
  const megabytes = Number(value);
  if (!Number.isFinite(megabytes)) {
    return 1024;
  }

  return Math.min(Math.max(Math.trunc(megabytes), 128), 10240);
}

export function shouldAskForBackgroundPermission(
  currentSettings: AppSettings,
  nextSettings: AppSettings,
): boolean {
  if (nextSettings.autoChangeMinutes <= 0) {
    return false;
  }

  if (nextSettings.runInBackground && nextSettings.launchAtStartup) {
    return false;
  }

  return (
    !currentSettings.runInBackground ||
    !currentSettings.launchAtStartup ||
    currentSettings.autoChangeMinutes <= 0
  );
}

export function withBackgroundPermission(settings: AppSettings): AppSettings {
  return {
    ...settings,
    runInBackground: true,
    launchAtStartup: true,
  };
}
