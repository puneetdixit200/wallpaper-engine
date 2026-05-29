import { invoke } from "@tauri-apps/api/core";
import { FormEvent, useEffect, useRef, useState } from "react";
import { Keyboard, Lock, ShieldCheck } from "lucide-react";
import { useAppState } from "../appState";
import { logAppAction } from "../appLog";
import { hotkeyCaptureFromKeyboardEvent } from "../hotkeyCapture";
import { AppSettings, QualityGuardMode } from "../types";

const qualityModes: Array<{ label: string; value: QualityGuardMode }> = [
  { label: "Off", value: "off" },
  { label: "Warn", value: "warn" },
  { label: "Skip", value: "skip" },
];

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function ControlsPage() {
  const { busy, settings, actions } = useAppState();
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
  }, [capturingHotkey]);

  function updateDraft(next: Partial<AppSettings>) {
    setDraft((current) => ({ ...current, ...next }));
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void logAppAction("controls.submit", "Controls form submitted.");
    void actions.saveSettings(draft);
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
        void logAppAction(
          "hotkeys.capture.pause",
          "Global hotkeys paused for capture.",
        );
      }
    } catch (error) {
      console.warn("Could not pause global hotkeys for capture", error);
    }
  }

  async function restoreGlobalHotkeysAfterCapture() {
    try {
      if (isTauriRuntime()) {
        await invoke("restore_global_hotkeys_after_capture");
        void logAppAction(
          "hotkeys.capture.restore",
          "Global hotkeys restored after capture.",
        );
      }
    } catch (error) {
      console.warn("Could not restore global hotkeys after capture", error);
    }
  }

  function startHotkeyCapture(key: keyof AppSettings["hotkeys"]) {
    void logAppAction("hotkeys.capture.start", "Hotkey capture started.", {
      hotkey: key,
    });
    committingHotkeyRef.current = false;
    captureValueRef.current = "";
    setCapturePreview("");
    setCapturingHotkey(key);
    void pauseGlobalHotkeysForCapture();
  }

  function resetHotkey(key: keyof AppSettings["hotkeys"]) {
    void logAppAction("hotkeys.capture.reset", "Hotkey capture reset.", {
      hotkey: key,
    });
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

    void logAppAction("hotkeys.capture.save", "Hotkey capture saved.", {
      hotkey: key,
      value,
    });
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
    void logAppAction("hotkeys.capture.cancel", "Hotkey capture cancelled.", {
      hotkey: key,
    });
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
          <p className="eyebrow">Controls</p>
          <h2>Quality guard and hotkeys</h2>
        </div>
      </header>

      <form className="settings-form" onSubmit={submit}>
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
                {draft.hotkeys.nextWallpaper} next, {draft.hotkeys.pauseRotation} pause,{" "}
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

        <div className="settings-actions">
          <button className="primary-button" disabled={busy === "settings"} type="submit">
            Save controls
          </button>
        </div>
      </form>
    </div>
  );
}
