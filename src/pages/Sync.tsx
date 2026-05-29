import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  SignedIn,
  SignedOut,
  useClerk,
  useSignIn,
  UserButton,
  useAuth,
  useUser,
} from "@clerk/clerk-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Cloud,
  Database,
  DownloadCloud,
  KeyRound,
  LogIn,
  ShieldCheck,
  UploadCloud,
  UserRound,
} from "lucide-react";
import { useAppState } from "../appState";
import {
  ClerkAuthEventDetail,
  clerkErrorMessage,
  desktopAuthBridgeUrl,
  externalClerkVerificationUrl,
} from "../clerkDesktopAuth";
import {
  ClerkAuthSettings,
  SupabaseSyncSettings,
  SupabaseSyncStatus,
  SyncAuthContext,
} from "../types";

const supabaseSchema = `create table if not exists public.wallpaper_engine_sync (
  id text primary key,
  payload jsonb not null,
  updated_at text not null
);

alter table public.wallpaper_engine_sync enable row level security;

create policy "Wallpaper Engine read own sync row"
on public.wallpaper_engine_sync
for select
to authenticated
using ((select auth.jwt() ->> 'sub') = id);

create policy "Wallpaper Engine insert own sync row"
on public.wallpaper_engine_sync
for insert
to authenticated
with check ((select auth.jwt() ->> 'sub') = id);

create policy "Wallpaper Engine update own sync row"
on public.wallpaper_engine_sync
for update
to authenticated
using ((select auth.jwt() ->> 'sub') = id)
with check ((select auth.jwt() ->> 'sub') = id);`;

type ConnectionState = "idle" | "ready" | "checking" | "connected" | "error";

interface ConnectionPanel {
  state: ConnectionState;
  message: string;
  updatedAt: string | null;
}

type SyncAction = (
  authContext?: SyncAuthContext | null,
) => Promise<SupabaseSyncStatus | null>;

interface SyncActionButtonsProps {
  canRunSync: boolean;
  clerkProviderReady: boolean;
  syncBusy: boolean;
  useClerkAuth: boolean;
  onStatus: (panel: ConnectionPanel) => void;
  onRun: (
    action: SyncAction,
    pendingMessage: string,
    authContext?: SyncAuthContext | null,
  ) => Promise<void>;
  testAction: SyncAction;
  pushAction: SyncAction;
  pullAction: SyncAction;
}

export function SyncPage() {
  const { busy, library, settings, actions } = useAppState();
  const [syncDraft, setSyncDraft] = useState(settings.supabaseSync);
  const [clerkDraft, setClerkDraft] = useState(settings.clerkAuth);
  const [connection, setConnection] = useState<ConnectionPanel>(() =>
    connectionPanelForSettings(settings.supabaseSync, settings.clerkAuth),
  );

  useEffect(() => {
    setSyncDraft(settings.supabaseSync);
    setClerkDraft(settings.clerkAuth);
  }, [settings.clerkAuth, settings.supabaseSync]);

  const providerKeyCount = useMemo(
    () =>
      Object.values(settings.apiKeys).filter((value) => value.trim().length > 0)
        .length,
    [settings.apiKeys],
  );
  const canRunSync =
    syncDraft.projectUrl.trim().length > 0 &&
    syncDraft.anonKey.trim().length > 0 &&
    (syncDraft.useClerkAuth || syncDraft.syncId.trim().length > 0);
  const syncBusy = busy?.startsWith("supabase") || busy === "settings";
  const savedConnectionKey = connectionKey(
    settings.supabaseSync,
    settings.clerkAuth,
  );
  const draftConnectionKey = connectionKey(syncDraft, clerkDraft);
  const hasUnsavedConnection = savedConnectionKey !== draftConnectionKey;
  const clerkProviderReady = isClerkProviderReady(settings.clerkAuth);
  const visibleConnection = hasUnsavedConnection
    ? {
        state: "ready" as const,
        message: "Save keys to test this connection.",
        updatedAt: null,
      }
    : connection;

  useEffect(() => {
    if (hasUnsavedConnection) {
      return;
    }
    const baseline = connectionPanelForSettings(
      settings.supabaseSync,
      settings.clerkAuth,
    );
    setConnection(baseline);
    if (baseline.state !== "ready" || settings.supabaseSync.useClerkAuth) {
      return;
    }

    let cancelled = false;
    setConnection({
      state: "checking",
      message: "Checking saved Supabase connection...",
      updatedAt: null,
    });
    void actions.testSupabaseSync(null).then((status) => {
      if (!cancelled) {
        setConnection(connectionPanelForStatus(status));
      }
    });
    return () => {
      cancelled = true;
    };
  }, [
    actions,
    hasUnsavedConnection,
    savedConnectionKey,
    settings.clerkAuth,
    settings.supabaseSync,
  ]);

  function updateSyncDraft(next: Partial<SupabaseSyncSettings>) {
    setSyncDraft((current) => ({ ...current, ...next }));
  }

  function updateClerkDraft(next: Partial<ClerkAuthSettings>) {
    setClerkDraft((current) => ({ ...current, ...next }));
  }

  async function saveDraft() {
    await actions.saveSettings({
      ...settings,
      clerkAuth: clerkDraft,
      supabaseSync: syncDraft,
    });
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void saveDraft();
  }

  async function runAfterSave(
    action: SyncAction,
    pendingMessage: string,
    authContext?: SyncAuthContext | null,
  ) {
    setConnection({
      state: "checking",
      message: pendingMessage,
      updatedAt: null,
    });
    await saveDraft();
    const status = await action(authContext ?? null);
    setConnection(connectionPanelForStatus(status));
  }

  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Sync</p>
          <h2>Supabase cloud sync</h2>
        </div>
      </header>

      <form className="settings-form sync-form" onSubmit={submit}>
        <section className="settings-section" aria-labelledby="sync-keys-heading">
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <KeyRound size={18} />
            </span>
            <div>
              <h3 id="sync-keys-heading">Connection keys</h3>
              <p>Store Supabase and auth connection details locally.</p>
            </div>
          </div>

          <label className="toggle-row">
            <input
              checked={syncDraft.enabled}
              onChange={(event) =>
                updateSyncDraft({ enabled: event.currentTarget.checked })
              }
              type="checkbox"
            />
            <span className="toggle-copy">
              <strong>
                <Cloud size={16} aria-hidden="true" />
                Enable Supabase sync
              </strong>
              <span>Push and pull settings, provider keys, favorites, and playlists.</span>
            </span>
          </label>

          <label className="toggle-row">
            <input
              checked={syncDraft.useClerkAuth}
              onChange={(event) =>
                updateSyncDraft({
                  enabled: event.currentTarget.checked || syncDraft.enabled,
                  useClerkAuth: event.currentTarget.checked,
                })
              }
              type="checkbox"
            />
            <span className="toggle-copy">
              <strong>
                <ShieldCheck size={16} aria-hidden="true" />
                Use Clerk login for sync
              </strong>
              <span>Recommended: the cloud row is locked to the signed-in Clerk user.</span>
            </span>
          </label>

          <div className="form-grid">
            <label>
              <span>Project URL</span>
              <input
                autoComplete="off"
                onChange={(event) =>
                  updateSyncDraft({ projectUrl: event.currentTarget.value })
                }
                placeholder="https://project.supabase.co"
                type="url"
                value={syncDraft.projectUrl}
              />
            </label>

            <label>
              <span>Anon key</span>
              <input
                autoComplete="off"
                onChange={(event) =>
                  updateSyncDraft({ anonKey: event.currentTarget.value })
                }
                placeholder="Paste Supabase anon key"
                type="password"
                value={syncDraft.anonKey}
              />
            </label>

            <label>
              <span>Sync ID</span>
              <input
                autoComplete="off"
                disabled={syncDraft.useClerkAuth}
                onChange={(event) =>
                  updateSyncDraft({ syncId: event.currentTarget.value })
                }
                placeholder="default"
                value={syncDraft.syncId}
              />
            </label>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="clerk-auth-heading">
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <UserRound size={18} />
            </span>
            <div>
              <h3 id="clerk-auth-heading">Clerk login</h3>
              <p>Save a Clerk publishable key, then sign in before syncing.</p>
            </div>
          </div>

          <label className="toggle-row">
            <input
              checked={clerkDraft.enabled}
              onChange={(event) =>
                updateClerkDraft({ enabled: event.currentTarget.checked })
              }
              type="checkbox"
            />
            <span className="toggle-copy">
              <strong>
                <LogIn size={16} aria-hidden="true" />
                Enable Clerk auth
              </strong>
              <span>Use the same login on every desktop install.</span>
            </span>
          </label>

          <div className="form-grid">
            <label>
              <span>Clerk publishable key</span>
              <input
                autoComplete="off"
                onChange={(event) =>
                  updateClerkDraft({ publishableKey: event.currentTarget.value })
                }
                placeholder="pk_live_..."
                type="password"
                value={clerkDraft.publishableKey}
              />
            </label>
          </div>

          <ClerkAccountPanel
            clerkProviderReady={clerkProviderReady}
            hasUnsavedConnection={hasUnsavedConnection}
          />
        </section>

        <section className="settings-section" aria-labelledby="sync-actions-heading">
          <div className="settings-section-heading">
            <span className="section-icon" aria-hidden="true">
              <Database size={18} />
            </span>
            <div>
              <h3 id="sync-actions-heading">Cloud snapshot</h3>
              <p>One row stores the current app state for this account.</p>
            </div>
          </div>

          <div className="sync-connection" data-state={visibleConnection.state}>
            <span className="sync-dot" aria-hidden="true" />
            <div>
              <strong>{connectionTitle(visibleConnection.state)}</strong>
              <span>{visibleConnection.message}</span>
              {visibleConnection.updatedAt ? (
                <small>Last cloud update: {visibleConnection.updatedAt}</small>
              ) : null}
            </div>
          </div>

          <div className="sync-status-grid">
            <div className="sync-stat">
              <span>Favorites</span>
              <strong>{library.favorites.length}</strong>
            </div>
            <div className="sync-stat">
              <span>Downloads</span>
              <strong>{library.downloaded.length}</strong>
            </div>
            <div className="sync-stat">
              <span>Playlists</span>
              <strong>{library.playlists.length}</strong>
            </div>
            <div className="sync-stat">
              <span>Provider keys</span>
              <strong>{providerKeyCount}</strong>
            </div>
          </div>

          <SyncActionButtons
            canRunSync={canRunSync}
            clerkProviderReady={clerkProviderReady}
            onRun={runAfterSave}
            onStatus={setConnection}
            pullAction={actions.pullSupabaseSync}
            pushAction={actions.pushSupabaseSync}
            syncBusy={syncBusy}
            testAction={actions.testSupabaseSync}
            useClerkAuth={syncDraft.useClerkAuth}
          />
        </section>

        <section className="settings-section" aria-labelledby="sync-schema-heading">
          <div className="settings-section-heading">
            <div>
              <h3 id="sync-schema-heading">Supabase table</h3>
              <p>Run this SQL after enabling Clerk as a Supabase third-party auth provider.</p>
            </div>
          </div>

          <pre className="code-block">
            <code>{supabaseSchema}</code>
          </pre>
        </section>
      </form>
    </div>
  );
}

function ClerkAccountPanel({
  clerkProviderReady,
  hasUnsavedConnection,
}: {
  clerkProviderReady: boolean;
  hasUnsavedConnection: boolean;
}) {
  if (hasUnsavedConnection || !clerkProviderReady) {
    return (
      <div className="sync-auth-card">
        <span className="sync-auth-icon" aria-hidden="true">
          <UserRound size={18} />
        </span>
        <div>
          <strong>Clerk sign-in inactive</strong>
          <span>Save a valid Clerk publishable key to activate login.</span>
        </div>
      </div>
    );
  }

  return <ClerkRuntimePanel />;
}

function ClerkRuntimePanel() {
  const clerk = useClerk();
  const { isLoaded, isSignedIn, userId } = useAuth();
  const {
    isLoaded: isSignInLoaded,
    setActive,
    signIn,
  } = useSignIn();
  const { user } = useUser();
  const [browserBusy, setBrowserBusy] = useState(false);
  const [browserMessage, setBrowserMessage] = useState("");
  const identity =
    user?.primaryEmailAddress?.emailAddress ??
    user?.fullName ??
    userId ??
    "Not signed in";

  useEffect(() => {
    function updateFromAuthEvent(event: Event) {
      const detail = (event as CustomEvent<ClerkAuthEventDetail>).detail;
      if (!detail) {
        return;
      }
      setBrowserBusy(false);
      setBrowserMessage(detail.message);
    }

    window.addEventListener("wallpaper-engine-clerk-auth", updateFromAuthEvent);
    return () =>
      window.removeEventListener(
        "wallpaper-engine-clerk-auth",
        updateFromAuthEvent,
      );
  }, []);

  async function startBrowserSignIn() {
    if (!clerk.loaded || !isSignInLoaded || !signIn) {
      setBrowserMessage("Clerk is still loading.");
      return;
    }

    setBrowserBusy(true);
    setBrowserMessage("Creating browser sign-in...");
    try {
      const signInAttempt = await signIn.create({
        strategy: "oauth_google",
        redirectUrl: desktopAuthBridgeUrl,
        actionCompleteRedirectUrl: desktopAuthBridgeUrl,
      });

      if (signInAttempt.status === "complete" && signInAttempt.createdSessionId) {
        await setActive({ session: signInAttempt.createdSessionId });
        setBrowserBusy(false);
        setBrowserMessage("Signed in with Clerk.");
        return;
      }

      await openUrl(externalClerkVerificationUrl(signInAttempt));
      setBrowserMessage("Continue sign-in in your browser.");
    } catch (error) {
      setBrowserBusy(false);
      setBrowserMessage(`Browser sign-in failed: ${clerkErrorMessage(error)}`);
    }
  }

  return (
    <div className="sync-auth-card" data-active={isSignedIn ? "true" : "false"}>
      <span className="sync-auth-icon" aria-hidden="true">
        <UserRound size={18} />
      </span>
      <div>
        <strong>
          {isLoaded && isSignedIn ? "Signed in with Clerk" : "Clerk sign-in ready"}
        </strong>
        <span>{isLoaded ? identity : "Loading Clerk session..."}</span>
      </div>
      <div className="sync-auth-actions">
        <SignedOut>
          <div className="browser-auth-actions">
            <button
              className="primary-button"
              disabled={browserBusy}
              onClick={() => void startBrowserSignIn()}
              type="button"
            >
              <LogIn size={16} aria-hidden="true" />
              {browserBusy ? "Opening..." : "Sign in with Google"}
            </button>
          </div>
        </SignedOut>
        <SignedIn>
          <UserButton />
        </SignedIn>
      </div>
      {browserMessage ? (
        <small className="sync-auth-message">{browserMessage}</small>
      ) : null}
    </div>
  );
}

function SyncActionButtons({
  canRunSync,
  clerkProviderReady,
  syncBusy,
  useClerkAuth,
  onStatus,
  onRun,
  testAction,
  pushAction,
  pullAction,
}: SyncActionButtonsProps) {
  if (useClerkAuth && clerkProviderReady) {
    return (
      <ClerkSyncActionButtons
        canRunSync={canRunSync}
        onRun={onRun}
        onStatus={onStatus}
        pullAction={pullAction}
        pushAction={pushAction}
        syncBusy={syncBusy}
        testAction={testAction}
      />
    );
  }

  const disabled = !canRunSync || syncBusy || useClerkAuth;

  return (
    <div className="settings-actions sync-actions">
      <button className="primary-button" disabled={syncBusy} type="submit">
        <Cloud size={16} aria-hidden="true" />
        Save keys
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void onRun(testAction, "Testing Supabase connection...", null)
        }
        type="button"
      >
        <Database size={16} aria-hidden="true" />
        Test
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void onRun(pushAction, "Pushing cloud snapshot...", null)
        }
        type="button"
      >
        <UploadCloud size={16} aria-hidden="true" />
        Push
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void onRun(pullAction, "Pulling cloud snapshot...", null)
        }
        type="button"
      >
        <DownloadCloud size={16} aria-hidden="true" />
        Pull
      </button>
    </div>
  );
}

function ClerkSyncActionButtons({
  canRunSync,
  syncBusy,
  onStatus,
  onRun,
  testAction,
  pushAction,
  pullAction,
}: Omit<SyncActionButtonsProps, "clerkProviderReady" | "useClerkAuth">) {
  const { getToken, isLoaded, isSignedIn, userId } = useAuth();
  const needsSignIn = !isLoaded || !isSignedIn || !userId;
  const disabled = !canRunSync || syncBusy || needsSignIn;

  async function runWithClerk(action: SyncAction, pendingMessage: string) {
    if (!isLoaded) {
      onStatus({
        state: "checking",
        message: "Loading Clerk session...",
        updatedAt: null,
      });
      return;
    }
    if (!isSignedIn || !userId) {
      onStatus({
        state: "error",
        message: "Sign in with Clerk before using Supabase sync.",
        updatedAt: null,
      });
      return;
    }
    const accessToken = await getToken();
    if (!accessToken) {
      onStatus({
        state: "error",
        message: "Clerk session token is missing. Sign in again and retry.",
        updatedAt: null,
      });
      return;
    }
    await onRun(action, pendingMessage, {
      accessToken,
      userId,
    });
  }

  return (
    <div className="settings-actions sync-actions">
      <button className="primary-button" disabled={syncBusy} type="submit">
        <Cloud size={16} aria-hidden="true" />
        Save keys
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void runWithClerk(testAction, "Testing Clerk Supabase connection...")
        }
        type="button"
      >
        <Database size={16} aria-hidden="true" />
        Test
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void runWithClerk(pushAction, "Pushing cloud snapshot...")
        }
        type="button"
      >
        <UploadCloud size={16} aria-hidden="true" />
        Push
      </button>
      <button
        className="secondary-button"
        disabled={disabled}
        onClick={() =>
          void runWithClerk(pullAction, "Pulling cloud snapshot...")
        }
        type="button"
      >
        <DownloadCloud size={16} aria-hidden="true" />
        Pull
      </button>
    </div>
  );
}

function connectionKey(
  config: SupabaseSyncSettings,
  clerkAuth: ClerkAuthSettings,
): string {
  return [
    config.enabled,
    config.projectUrl.trim(),
    config.anonKey.trim(),
    config.useClerkAuth,
    config.syncId.trim(),
    clerkAuth.enabled,
    clerkAuth.publishableKey.trim(),
  ].join("|");
}

function isClerkProviderReady(clerkAuth: ClerkAuthSettings): boolean {
  return clerkAuth.enabled && clerkAuth.publishableKey.trim().length > 0;
}

function connectionPanelForSettings(
  config: SupabaseSyncSettings,
  clerkAuth: ClerkAuthSettings,
): ConnectionPanel {
  if (!config.enabled) {
    return {
      state: "idle",
      message: "Supabase sync is off.",
      updatedAt: null,
    };
  }
  if (!config.projectUrl.trim() || !config.anonKey.trim()) {
    return {
      state: "idle",
      message: "Add the project URL and anon key.",
      updatedAt: null,
    };
  }
  if (config.useClerkAuth) {
    if (!isClerkProviderReady(clerkAuth)) {
      return {
        state: "idle",
        message: "Add and save a Clerk publishable key before signing in.",
        updatedAt: null,
      };
    }
    return {
      state: "ready",
      message: "Sign in with Clerk, then test the saved connection.",
      updatedAt: null,
    };
  }
  if (!config.syncId.trim()) {
    return {
      state: "idle",
      message: "Add the project URL, anon key, and Sync ID.",
      updatedAt: null,
    };
  }
  return {
    state: "ready",
    message: "Ready to test the saved connection.",
    updatedAt: null,
  };
}

function connectionPanelForStatus(
  status: SupabaseSyncStatus | null,
): ConnectionPanel {
  if (!status) {
    return {
      state: "error",
      message: "No response came back from the sync command.",
      updatedAt: null,
    };
  }
  return {
    state: status.connected ? "connected" : "error",
    message: status.message,
    updatedAt: status.updatedAt,
  };
}

function connectionTitle(state: ConnectionState): string {
  switch (state) {
    case "connected":
      return "Connected";
    case "checking":
      return "Checking";
    case "error":
      return "Not connected";
    case "ready":
      return "Ready";
    case "idle":
      return "Not connected";
  }
}
