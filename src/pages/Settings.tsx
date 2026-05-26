import { FormEvent, useEffect, useState } from "react";
import { useAppState } from "../appState";
import { runConfirmed } from "../confirmAction";
import {
  AppSettings,
  ResolutionPreference,
  ThemePreference,
  WallpaperLayoutPreference,
} from "../types";
import { parseAutoChangeMinutes, parseCacheLimitMb } from "../settingsFlow";

const resolutions: Array<{ label: string; value: ResolutionPreference }> = [
  { label: "Auto", value: "auto" },
  { label: "1080p", value: "fullHd" },
  { label: "4K", value: "fourK" },
];

const themes: Array<{ label: string; value: ThemePreference }> = [
  { label: "System", value: "system" },
  { label: "Light", value: "light" },
  { label: "Dark", value: "dark" },
];

const wallpaperLayouts: Array<{
  label: string;
  value: WallpaperLayoutPreference;
}> = [
  { label: "Fill", value: "fill" },
  { label: "Fit", value: "fit" },
  { label: "Stretch", value: "stretch" },
  { label: "Tile", value: "tile" },
  { label: "Center", value: "center" },
  { label: "Span", value: "span" },
];

export function SettingsPage() {
  const { busy, cacheStats, settings, actions } = useAppState();
  const [draft, setDraft] = useState(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  function updateDraft(next: Partial<AppSettings>) {
    setDraft((current) => ({ ...current, ...next }));
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void actions.saveSettings(draft);
  }

  function updateApiKey(key: keyof AppSettings["apiKeys"], value: string) {
    updateDraft({
      apiKeys: {
        ...draft.apiKeys,
        [key]: value,
      },
    });
  }

  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>API keys and wallpaper behavior</h2>
        </div>
      </header>

      <form className="settings-form" onSubmit={submit}>
        <label>
          <span>Pexels API key</span>
          <input
            autoComplete="off"
            onChange={(event) => updateApiKey("pexels", event.currentTarget.value)}
            placeholder="Paste Pexels key"
            type="password"
            value={draft.apiKeys.pexels}
          />
        </label>

        <label>
          <span>Unsplash API key</span>
          <input
            autoComplete="off"
            onChange={(event) => updateApiKey("unsplash", event.currentTarget.value)}
            placeholder="Paste Unsplash access key"
            type="password"
            value={draft.apiKeys.unsplash}
          />
        </label>

        <div className="form-grid">
          <label>
            <span>Pixabay API key</span>
            <input
              autoComplete="off"
              onChange={(event) => updateApiKey("pixabay", event.currentTarget.value)}
              placeholder="Paste Pixabay key"
              type="password"
              value={draft.apiKeys.pixabay}
            />
          </label>

          <label>
            <span>Wallhaven API key</span>
            <input
              autoComplete="off"
              onChange={(event) => updateApiKey("wallhaven", event.currentTarget.value)}
              placeholder="Required for NSFW"
              type="password"
              value={draft.apiKeys.wallhaven}
            />
          </label>

          <label>
            <span>DeviantArt access token</span>
            <input
              autoComplete="off"
              onChange={(event) => updateApiKey("deviantart", event.currentTarget.value)}
              placeholder="OAuth access token"
              type="password"
              value={draft.apiKeys.deviantart}
            />
          </label>
        </div>

        <div className="form-grid">
          <label>
            <span>Theme</span>
            <select
              onChange={(event) =>
                updateDraft({
                  theme: event.currentTarget.value as ThemePreference,
                })
              }
              value={draft.theme}
            >
              {themes.map((theme) => (
                <option key={theme.value} value={theme.value}>
                  {theme.label}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Wallpaper layout</span>
            <select
              onChange={(event) =>
                updateDraft({
                  wallpaperLayout: event.currentTarget
                    .value as WallpaperLayoutPreference,
                })
              }
              value={draft.wallpaperLayout}
            >
              {wallpaperLayouts.map((layout) => (
                <option key={layout.value} value={layout.value}>
                  {layout.label}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Auto-change minutes</span>
            <input
              min={0}
              max={1440}
              onChange={(event) =>
                updateDraft({
                  autoChangeMinutes: parseAutoChangeMinutes(
                    event.currentTarget.value,
                  ),
                })
              }
              step={1}
              type="number"
              value={draft.autoChangeMinutes}
            />
          </label>

          <label>
            <span>Resolution</span>
            <select
              onChange={(event) =>
                updateDraft({
                  resolution: event.currentTarget.value as ResolutionPreference,
                })
              }
              value={draft.resolution}
            >
              {resolutions.map((resolution) => (
                <option key={resolution.value} value={resolution.value}>
                  {resolution.label}
                </option>
              ))}
            </select>
          </label>

          <label>
            <span>Cache limit MB</span>
            <input
              min={128}
              max={10240}
              onChange={(event) =>
                updateDraft({
                  cacheLimitMb: parseCacheLimitMb(event.currentTarget.value),
                })
              }
              step={128}
              type="number"
              value={draft.cacheLimitMb}
            />
          </label>
        </div>

        <label className="checkbox-row">
          <input
            checked={draft.allowNsfwWallhaven}
            onChange={(event) =>
              updateDraft({ allowNsfwWallhaven: event.currentTarget.checked })
            }
            type="checkbox"
          />
          <span>Allow Wallhaven NSFW</span>
        </label>

        <div className="settings-actions">
          <button className="primary-button" disabled={busy === "settings"} type="submit">
            Save settings
          </button>
          <button
            className="secondary-button"
            disabled={busy === "clear-cache"}
            onClick={() =>
              void runConfirmed(
                (message) => window.confirm(message),
                "Clear every cached wallpaper file?",
                actions.clearWallpaperCache,
              )
            }
            type="button"
          >
            Clear cache
          </button>
          <span>
            {(cacheStats.bytes / 1024 / 1024).toFixed(1)} MB, {cacheStats.files} files
          </span>
        </div>
      </form>
    </div>
  );
}
