import { useAppState } from "../appState";
import { runConfirmed } from "../confirmAction";
import { EmptyState } from "../components/EmptyState";
import { WallCard } from "../components/WallCard";

export function LibraryPage() {
  const { busy, cacheStats, library, actions } = useAppState();
  const hasLibraryItems =
    library.favorites.length > 0 || library.downloaded.length > 0;
  const hasCachedDownloads = cacheStats.files > 0;
  const canClearLibrary = hasLibraryItems || hasCachedDownloads;

  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Library</p>
          <h2>Saved and downloaded wallpapers</h2>
        </div>
        <button
          className="secondary-button"
          disabled={!canClearLibrary || busy === "clear-library"}
          onClick={() =>
            void runConfirmed(
              (message) => window.confirm(message),
              "Clear all saved, downloaded, and cached wallpaper files?",
              actions.clearLibrary,
            )
          }
          type="button"
        >
          Clear library
        </button>
      </header>

      <section className="section-band">
        <div className="section-title">
          <h3>Saved walls</h3>
          <span>{library.favorites.length}</span>
        </div>
        <div className="wall-grid">
          {library.favorites.length === 0 ? (
            <EmptyState
              title="No favorites yet"
              detail="Saved wallpapers will appear here."
            />
          ) : null}
          {library.favorites.map((wallpaper) => (
            <WallCard key={wallpaper.id} wallpaper={wallpaper} canDelete />
          ))}
        </div>
      </section>

      <section className="section-band">
        <div className="section-title">
          <h3>Downloaded</h3>
          <span>{library.downloaded.length}</span>
        </div>
        <div className="wall-grid">
          {library.downloaded.length === 0 ? (
            <EmptyState
              title="No downloads yet"
              detail="Cached wallpapers will appear here."
            />
          ) : null}
          {library.downloaded.map((wallpaper) => (
            <WallCard key={wallpaper.id} wallpaper={wallpaper} canDelete />
          ))}
        </div>
      </section>
    </div>
  );
}
