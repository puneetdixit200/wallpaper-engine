import { invoke } from "@tauri-apps/api/core";
import { FormEvent, useEffect, useRef, useState } from "react";
import { Clock, Keyboard, Lock, Power, Rocket, ShieldCheck } from "lucide-react";
import { useAppState } from "../appState";
import { runConfirmed } from "../confirmAction";
import { hotkeyCaptureFromKeyboardEvent } from "../hotkeyCapture";
import {
  AppSettings,
  QualityGuardMode,
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

const qualityModes: Array<{ label: string; value: QualityGuardMode }> = [
  { label: "Off", value: "off" },
  { label: "Warn", value: "warn" },
  { label: "Skip", value: "skip" },
];

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function SettingsPage() {
  const { busy, cacheStats, library, settings, actions } = useAppState();
  const [draft, setDraft] = useState(settings);
  const [capturingHotkey, setCapturingHotkey] = useState<
    keyof AppSettings["hotkeys"] | null
  >(null);
  const [capturePreview, setCapturePreview] = useState("");
  const committingHotkeyRef = useRef(false);
  const captureValueRef = useRef("");

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    if (!capturingHotkey) {
      return;
    }

    const handleKeyDown = (event: globalThis.KeyboardEvent) => {
      handleHotkeyCapture(event, capturingHotkey);
    };
    document.addEventListener("keydown", handleKeyDown, true);
    return () => document.removeEventListener("keydown", handleKeyDown, true);
  });

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

  function updateHotkey(key: keyof AppSettings["hotkeys"], value: string) {
    setDraft((current) => ({
      ...current,
      hotkeys: {
        ...current.hotkeys,
        [key]: value,
      },
    }));
  }

  async function pauseGlobalHotkeysForCapture() {
    try {
      if (isTauriRuntime()) {
        await invoke("pause_global_hotkeys_for_capture");
      }
    } catch (error) {
      console.warn("Could not pause global hotkeys for capture", error);
    }
  }

  async function restoreGlobalHotkeysAfterCapture() {
    try {
      if (isTauriRuntime()) {
        await invoke("restore_global_hotkeys_after_capture");
      }
    } catch (error) {
      console.warn("Could not restore global hotkeys after capture", error);
    }
  }

  function startHotkeyCapture(key: keyof AppSettings["hotkeys"]) {
    committingHotkeyRef.current = false;
    captureValueRef.current = "";
    setCapturePreview("");
    setCapturingHotkey(key);
    void pauseGlobalHotkeysForCapture();
  }

  function resetHotkey(key: keyof AppSettings["hotkeys"]) {
    setDraft((current) => ({
      ...current,
      hotkeys: {
        ...current.hotkeys,
        [key]: settings.hotkeys[key],
      },
    }));
    captureValueRef.current = "";
    setCapturePreview("");
    setCapturingHotkey(null);
    void restoreGlobalHotkeysAfterCapture();
  }

  function saveHotkeyCapture(key: keyof AppSettings["hotkeys"]) {
    const value = captureValueRef.current;
    if (!value) {
      return;
    }

    const nextSettings: AppSettings = {
      ...draft,
      hotkeys: {
        ...draft.hotkeys,
        [key]: value,
      },
    };
    committingHotkeyRef.current = true;
    setDraft(nextSettings);
    setCapturePreview("");
    setCapturingHotkey(null);
    void actions
      .saveSettings(nextSettings)
      .finally(() => void restoreGlobalHotkeysAfterCapture());
  }

  function leaveHotkeyCapture(key: keyof AppSettings["hotkeys"]) {
    if (capturingHotkey !== key) {
      return;
    }
    if (committingHotkeyRef.current) {
      setCapturingHotkey(null);
      setCapturePreview("");
      captureValueRef.current = "";
      committingHotkeyRef.current = false;
      return;
    }
    setDraft((current) => ({
      ...current,
      hotkeys: {
        ...current.hotkeys,
        [key]: settings.hotkeys[key],
      },
    }));
    setCapturingHotkey(null);
    setCapturePreview("");
    captureValueRef.current = "";
    void restoreGlobalHotkeysAfterCapture();
  }

  function handleHotkeyCapture(
    event: globalThis.KeyboardEvent,
    key: keyof AppSettings["hotkeys"],
  ) {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      resetHotkey(key);
      return;
    }

    if (event.key === "Enter") {
      saveHotkeyCapture(key);
      return;
    }

    const snapshot = hotkeyCaptureFromKeyboardEvent(event);
    if (snapshot) {
      setCapturePreview(snapshot.displayString);
      if (snapshot.value) {
        captureValueRef.current = snapshot.value;
        updateHotkey(key, snapshot.value);
      }
    }
  }

  function renderHotkeyCapture(
    key: keyof AppSettings["hotkeys"],
    label: string,
  ) {
    const isCapturing = capturingHotkey === key;
    return (
      <div className="hotkey-field">
        <span>{label}</span>
        <button
          className={isCapturing ? "hotkey-capture active" : "hotkey-capture"}
          onBlur={() => leaveHotkeyCapture(key)}
          onClick={() => startHotkeyCapture(key)}
          type="button"
        >
          {isCapturing
            ? capturePreview || "Press shortcut"
            : draft.hotkeys[key] || "Record hotkey"}
        </button>
        <span className="hotkey-hint">
          {isCapturing
            ? captureValueRef.current
              ? "Enter saves. Escape resets."
              : "Press modifiers, then one key. Enter saves when a full combo appears."
            : "Click to record a shortcut."}
        </span>
      </div>
    );
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

          <div className="form-grid two-column">
            <label>
              <span>Auto-change source</span>
              <select
                onChange={(event) =>
                  updateDraft({
                    activePlaylistId: event.currentTarget.value || null,
                  })
                }
                value={draft.activePlaylistId ?? ""}
              >
                <option value="">All providers</option>
                {library.playlists.map((playlist) => (
                  <option key={playlist.id} value={playlist.id}>
                    {playlist.name}
                  </option>
                ))}
              </select>
            </label>

            <div className="action-row compact-actions">
              <button
                className="secondary-button"
                disabled={busy === "next"}
                onClick={() => void actions.applyNextWallpaper()}
                type="button"
              >
                Next wallpaper
              </button>
              <button
                className="secondary-button"
                disabled={busy === "pause"}
                onClick={() => void actions.toggleAutoChangePause()}
                type="button"
              >
                Pause timer
              </button>
            </div>
          </div>
        </section>

        <section
          aria-labelledby="quality-heading"
          className="settings-section"
        >
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <ShieldCheck size={18} />
            </span>
            <div>
              <h3 id="quality-heading">Quality guard</h3>
              <p>Block or warn before low-resolution images are applied.</p>
            </div>
          </div>

          <div className="form-grid">
            <label>
              <span>Guard mode</span>
              <select
                onChange={(event) =>
                  updateDraft({
                    qualityGuardMode: event.currentTarget
                      .value as QualityGuardMode,
                  })
                }
                value={draft.qualityGuardMode}
              >
                {qualityModes.map((mode) => (
                  <option key={mode.value} value={mode.value}>
                    {mode.label}
                  </option>
                ))}
              </select>
            </label>

            <label>
              <span>Minimum width</span>
              <input
                min={320}
                max={15360}
                onChange={(event) =>
                  updateDraft({
                    qualityMinWidth:
                      Number.parseInt(event.currentTarget.value, 10) || 1920,
                  })
                }
                step={100}
                type="number"
                value={draft.qualityMinWidth}
              />
            </label>

            <label>
              <span>Minimum height</span>
              <input
                min={240}
                max={8640}
                onChange={(event) =>
                  updateDraft({
                    qualityMinHeight:
                      Number.parseInt(event.currentTarget.value, 10) || 1080,
                  })
                }
                step={100}
                type="number"
                value={draft.qualityMinHeight}
              />
            </label>
          </div>

          <div className="toggle-grid">
            <label className="toggle-row">
              <input
                checked={draft.allowPortraitWallpapers}
                onChange={(event) =>
                  updateDraft({
                    allowPortraitWallpapers: event.currentTarget.checked,
                  })
                }
                type="checkbox"
              />
              <span className="toggle-copy">
                <strong>Allow portrait wallpapers</strong>
                <span>Skip this warning when your display is vertical.</span>
              </span>
            </label>

            <label className="toggle-row">
              <input
                checked={draft.applyToLockScreen}
                onChange={(event) =>
                  updateDraft({
                    applyToLockScreen: event.currentTarget.checked,
                  })
                }
                type="checkbox"
              />
              <span className="toggle-copy">
                <strong>
                  <Lock size={16} aria-hidden="true" />
                  Also update lock screen
                </strong>
                <span>Uses OS support when the current platform allows it.</span>
              </span>
            </label>
          </div>
        </section>

        <section
          aria-labelledby="hotkeys-heading"
          className="settings-section"
        >
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <Keyboard size={18} />
            </span>
            <div>
              <h3 id="hotkeys-heading">Tray and global hotkeys</h3>
              <p>Use quick actions without opening the window.</p>
            </div>
          </div>

          <label className="toggle-row">
            <input
              checked={draft.globalHotkeysEnabled}
              onChange={(event) =>
                updateDraft({
                  globalHotkeysEnabled: event.currentTarget.checked,
                })
              }
              type="checkbox"
            />
            <span className="toggle-copy">
              <strong>Enable global hotkeys</strong>
              <span>
                {draft.hotkeys.nextWallpaper} next, {draft.hotkeys.pauseRotation} pause,
                {" "}
                {draft.hotkeys.favoriteCurrent} favorite current.
              </span>
            </span>
          </label>

          <div className="form-grid">
            {renderHotkeyCapture("nextWallpaper", "Next wallpaper hotkey")}
            {renderHotkeyCapture("pauseRotation", "Pause timer hotkey")}
            {renderHotkeyCapture("favoriteCurrent", "Favorite current hotkey")}
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

          <label>
            <span>Auto-clean after days</span>
            <input
              min={0}
              max={365}
              onChange={(event) =>
                updateDraft({
                  autoCleanDays:
                    Number.parseInt(event.currentTarget.value, 10) || 0,
                })
              }
              step={1}
              type="number"
              value={draft.autoCleanDays}
            />
          </label>
        </div>

        <div className="toggle-grid">
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

          <label className="checkbox-row">
            <input
              checked={draft.autoCleanKeepFavorites}
              onChange={(event) =>
                updateDraft({
                  autoCleanKeepFavorites: event.currentTarget.checked,
                })
              }
              type="checkbox"
            />
            <span>Keep favorites during auto-clean</span>
          </label>
        </div>
        </section>

        <div className="settings-actions">
          <button className="primary-button" disabled={busy === "settings"} type="submit">
            Save settings
          </button>
          <button
            className="secondary-button"
            disabled={busy === "auto-clean"}
            onClick={() => void actions.runAutoCleanup()}
            type="button"
          >
            Run auto-clean
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

        <footer className="settings-credit">
          Made with ❤️ by{" "}
          <a
            href="https://github.com/puneetdixit200"
            rel="noreferrer"
            target="_blank"
          >
            puneetdixit
          </a>
        </footer>
      </form>
    </div>
  );
}
