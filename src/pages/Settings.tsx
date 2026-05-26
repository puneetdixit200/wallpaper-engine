import { FormEvent, useEffect, useState } from "react";
import {
  CacheStats,
  AppSettings,
  ResolutionPreference,
  ThemePreference,
} from "../types";

interface SettingsPageProps {
  busy: string | null;
  cacheStats: CacheStats;
  settings: AppSettings;
  onClearCache: () => void;
  onSave: (settings: AppSettings) => void;
}

const intervals = [
  { label: "Off", value: 0 },
  { label: "15 min", value: 15 },
  { label: "30 min", value: 30 },
  { label: "1 hr", value: 60 },
  { label: "2 hr", value: 120 },
  { label: "6 hr", value: 360 },
];

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

export function SettingsPage({
  busy,
  cacheStats,
  settings,
  onClearCache,
  onSave,
}: SettingsPageProps) {
  const [draft, setDraft] = useState(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  function updateDraft(next: Partial<AppSettings>) {
    setDraft((current) => ({ ...current, ...next }));
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSave(draft);
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
            <span>Auto-change</span>
            <select
              onChange={(event) =>
                updateDraft({ autoChangeMinutes: Number(event.currentTarget.value) })
              }
              value={draft.autoChangeMinutes}
            >
              {intervals.map((interval) => (
                <option key={interval.value} value={interval.value}>
                  {interval.label}
                </option>
              ))}
            </select>
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
                updateDraft({ cacheLimitMb: Number(event.currentTarget.value) })
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
            onClick={onClearCache}
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
