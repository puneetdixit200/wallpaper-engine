import { ThemePreference } from "./types";

export type ResolvedTheme = "light" | "dark";

export function resolveThemePreference(
  theme: ThemePreference,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (theme === "system") {
    return systemPrefersDark ? "dark" : "light";
  }

  return theme;
}
