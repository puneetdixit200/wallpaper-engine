import { FormEvent, useEffect, useState } from "react";
import { Clock, Power, Rocket } from "lucide-react";
import { useAppState } from "../appState";
import { runConfirmed } from "../confirmAction";
import {
  AppSettings,
  ResolutionPreference,
  ThemePreference,
  WallpaperLayoutPreference,
} from "../types";
import {
  backgroundPermissionMessage,
  parseAutoChangeMinutes,
  parseCacheLimitMb,
  shouldAskForBackgroundPermission,
  withBackgroundPermission,
} from "../settingsFlow";

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

const autoChangePresets = [
  { label: "Off", value: 0 },
  { label: "15 min", value: 15 },
  { label: "30 min", value: 30 },
  { label: "1 hour", value: 60 },
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
    let nextSettings = draft;
    if (shouldAskForBackgroundPermission(settings, draft)) {
      const allowed = window.confirm(backgroundPermissionMessage);
      if (!allowed) {
        return;
      }
      nextSettings = withBackgroundPermission(draft);
      setDraft(nextSettings);
    }

    void actions.saveSettings(nextSettings);
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
        <section
          aria-labelledby="automation-heading"
          className="settings-section automation-section"
        >
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <Clock size={18} />
            </span>
            <div>
              <h3 id="automation-heading">Automatic wallpaper changes</h3>
              <p>
                Change wallpapers on a schedule and keep the timer alive after
                the window closes.
              </p>
            </div>
          </div>

          <div className="automation-controls">
            <div
              aria-label="Auto-change interval"
              className="automation-presets"
            >
              {autoChangePresets.map((preset) => (
                <button
                  aria-pressed={draft.autoChangeMinutes === preset.value}
                  className={
                    draft.autoChangeMinutes === preset.value
                      ? "automation-preset active"
                      : "automation-preset"
                  }
                  key={preset.value}
                  onClick={() =>
                    updateDraft({ autoChangeMinutes: preset.value })
                  }
                  type="button"
                >
                  {preset.label}
                </button>
              ))}
            </div>

            <label className="automation-number">
              <span>Change every</span>
              <div className="number-input-wrap">
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
                <span>minutes</span>
              </div>
            </label>
          </div>

          <div className="toggle-grid">
            <label className="toggle-row">
              <input
                checked={draft.runInBackground}
                onChange={(event) =>
                  updateDraft({ runInBackground: event.currentTarget.checked })
                }
                type="checkbox"
              />
              <span className="toggle-copy">
                <strong>
                  <Power size={16} aria-hidden="true" />
                  Keep changing after close
                </strong>
                <span>Hide to tray instead of stopping the scheduler.</span>
              </span>
            </label>

            <label className="toggle-row">
              <input
                checked={draft.launchAtStartup}
                onChange={(event) =>
                  updateDraft({ launchAtStartup: event.currentTarget.checked })
                }
                type="checkbox"
              />
              <span className="toggle-copy">
                <strong>
                  <Rocket size={16} aria-hidden="true" />
                  Start at login
                </strong>
                <span>Launch hidden when background mode is enabled.</span>
              </span>
            </label>
          </div>
        </section>

        <section
          aria-labelledby="api-keys-heading"
          className="settings-section"
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="api-keys-heading">Provider keys</h3>
            </div>
          </div>

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
        </section>

        <section
          aria-labelledby="appearance-heading"
          className="settings-section"
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="appearance-heading">Wallpaper and storage</h3>
            </div>
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
        </section>

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
