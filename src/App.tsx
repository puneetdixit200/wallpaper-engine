import { useEffect, useState } from "react";
import { Database, Heart, Home, Image, Search, Settings } from "lucide-react";
import { AppStateProvider, useAppState } from "./appState";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { HomePage } from "./pages/Home";
import { SearchPage } from "./pages/Search";
import { LibraryPage } from "./pages/Library";
import { SettingsPage } from "./pages/Settings";
import { resolveThemePreference } from "./themePreference";
import { ViewName } from "./types";
import "./App.css";

const navItems: Array<{ id: ViewName; label: string; icon: typeof Home }> = [
  { id: "home", label: "Home", icon: Home },
  { id: "search", label: "Search", icon: Search },
  { id: "library", label: "Library", icon: Image },
  { id: "settings", label: "Settings", icon: Settings },
];

function useSystemPrefersDark() {
  const [prefersDark, setPrefersDark] = useState(() =>
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setPrefersDark(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  return prefersDark;
}

function AppShell() {
  const {
    activeView,
    favoriteIds,
    settings,
    actions,
  } = useAppState();
  const systemPrefersDark = useSystemPrefersDark();
  const resolvedTheme = resolveThemePreference(
    settings.theme,
    systemPrefersDark,
  );

  const content =
    activeView === "home" ? (
      <HomePage />
    ) : activeView === "search" ? (
      <SearchPage />
    ) : activeView === "library" ? (
      <LibraryPage />
    ) : (
      <SettingsPage />
    );

  return (
    <main
      className="app-shell"
      data-theme={settings.theme}
      data-resolved-theme={resolvedTheme}
    >
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <Database size={18} aria-hidden="true" />
          </div>
          <div>
            <h1>Wallpaper Engine</h1>
            <p>Desktop wallpaper control</p>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                className={
                  activeView === item.id ? "nav-item active" : "nav-item"
                }
                key={item.id}
                onClick={() => actions.setActiveView(item.id)}
                type="button"
              >
                <Icon size={18} aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-footer">
          <Heart size={16} aria-hidden="true" />
          <span>{favoriteIds.size} saved</span>
        </div>
      </aside>

      <section className="content-shell">
        <div className="view-transition" key={activeView}>
          {content}
        </div>
      </section>
    </main>
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AppStateProvider>
        <AppShell />
      </AppStateProvider>
    </ErrorBoundary>
  );
}

export default App;
