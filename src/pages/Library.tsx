import { open } from "@tauri-apps/plugin-dialog";
import { FormEvent, useState } from "react";
import { Archive, FolderDown, ListPlus, Play, Trash2 } from "lucide-react";
import { useAppState } from "../appState";
import { EmptyState } from "../components/EmptyState";
import { WallCard } from "../components/WallCard";

export function LibraryPage() {
  const { busy, cacheStats, library, settings, actions } = useAppState();
  const [playlistName, setPlaylistName] = useState("");
  const [importFolderPath, setImportFolderPath] = useState("");
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [importSummary, setImportSummary] = useState("");
  const hasLibraryItems =
    library.favorites.length > 0 ||
    library.downloaded.length > 0 ||
    library.playlists.length > 0;
  const hasCachedDownloads = cacheStats.files > 0;
  const canClearLibrary = hasLibraryItems || hasCachedDownloads;

  async function createPlaylist(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!playlistName.trim()) {
      return;
    }
    await actions.createPlaylist(playlistName);
    setPlaylistName("");
  }

  async function importLocalFolderPath(folderPath: string) {
    if (!folderPath.trim()) {
      return;
    }
    const result = await actions.importLocalFolder(folderPath);
    if (result) {
      setImportSummary(`${result.imported} imported, ${result.skipped} skipped`);
    }
  }

  async function selectAndImportFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select wallpaper folder",
      });
      if (typeof selected !== "string") {
        return;
      }
      setImportFolderPath(selected);
      await importLocalFolderPath(selected);
    } catch (error) {
      setImportSummary(String(error));
    }
  }

  async function exportBackup(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (exportPath.trim()) {
      await actions.exportBackup(exportPath);
    }
  }

  async function importBackup(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (importPath.trim()) {
      await actions.importBackup(importPath);
    }
  }

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
          onClick={() => void actions.clearLibrary()}
          type="button"
        >
          Clear library
        </button>
      </header>

      <section className="section-band">
        <div className="section-title">
          <h3>Playlists</h3>
          <span>{library.playlists.length}</span>
        </div>

        <div className="tool-strip">
          <form className="inline-tool" onSubmit={createPlaylist}>
            <label>
              <span>New playlist</span>
              <input
                onChange={(event) => setPlaylistName(event.currentTarget.value)}
                placeholder="Favorites for work"
                value={playlistName}
              />
            </label>
            <button
              className="primary-button"
              disabled={busy === "playlist" || !playlistName.trim()}
              type="submit"
            >
              <ListPlus size={17} aria-hidden="true" />
              Create
            </button>
          </form>

          <div className="inline-tool">
            <label>
              <span>Auto-change source</span>
              <select
                onChange={(event) =>
                  void actions.saveSettings({
                    ...settings,
                    activePlaylistId: event.currentTarget.value || null,
                  })
                }
                value={settings.activePlaylistId ?? ""}
              >
                <option value="">All providers</option>
                {library.playlists.map((playlist) => (
                  <option key={playlist.id} value={playlist.id}>
                    {playlist.name}
                  </option>
                ))}
              </select>
            </label>
            <button
              className="secondary-button"
              disabled={busy === "next"}
              onClick={() => void actions.applyNextWallpaper()}
              type="button"
            >
              <Play size={17} aria-hidden="true" />
              Next
            </button>
          </div>
        </div>

        {library.playlists.length === 0 ? (
          <EmptyState
            title="No playlists yet"
            detail="Create a playlist, then add wallpapers from search or library cards."
          />
        ) : null}

        {library.playlists.map((playlist) => (
          <section className="playlist-section" key={playlist.id}>
            <div className="section-title">
              <h3>{playlist.name}</h3>
              <div className="title-actions">
                <span>{playlist.wallpapers.length}</span>
                <button
                  aria-label={`Delete ${playlist.name}`}
                  className="icon-button danger"
                  disabled={busy === `playlist-${playlist.id}`}
                  onClick={() => void actions.deletePlaylist(playlist.id)}
                  title="Delete playlist"
                  type="button"
                >
                  <Trash2 size={17} aria-hidden="true" />
                </button>
              </div>
            </div>
            <div className="wall-grid">
              {playlist.wallpapers.length === 0 ? (
                <EmptyState
                  title="Playlist is empty"
                  detail="Use the list button on any wallpaper card to add it."
                />
              ) : null}
              {playlist.wallpapers.map((wallpaper) => (
                <div className="playlist-wall" key={wallpaper.id}>
                  <WallCard wallpaper={wallpaper} />
                  <button
                    className="secondary-button compact-button"
                    disabled={busy === `playlist-${wallpaper.id}`}
                    onClick={() =>
                      void actions.removeWallpaperFromPlaylist(
                        playlist.id,
                        wallpaper.id,
                      )
                    }
                    type="button"
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          </section>
        ))}
      </section>

      <section className="section-band">
        <div className="section-title">
          <h3>Import and backup</h3>
          <span>{(cacheStats.bytes / 1024 / 1024).toFixed(1)} MB cached</span>
        </div>
        <div className="tool-strip">
          <div className="inline-tool">
            <label>
              <span>Local folder path</span>
              <input
                onChange={(event) => setImportFolderPath(event.currentTarget.value)}
                placeholder="/Users/name/Pictures/Wallpapers"
                value={importFolderPath}
              />
            </label>
            <button
              className="secondary-button"
              disabled={busy === "import-folder"}
              onClick={() => void selectAndImportFolder()}
              type="button"
            >
              <FolderDown size={17} aria-hidden="true" />
              Import
            </button>
          </div>
          {importSummary ? <p className="tool-note">{importSummary}</p> : null}

          <form className="inline-tool" onSubmit={exportBackup}>
            <label>
              <span>Export backup path</span>
              <input
                onChange={(event) => setExportPath(event.currentTarget.value)}
                placeholder="/Users/name/Downloads/wallpaper-backup.json"
                value={exportPath}
              />
            </label>
            <button
              className="secondary-button"
              disabled={busy === "backup" || !exportPath.trim()}
              type="submit"
            >
              <Archive size={17} aria-hidden="true" />
              Export
            </button>
          </form>

          <form className="inline-tool" onSubmit={importBackup}>
            <label>
              <span>Import backup path</span>
              <input
                onChange={(event) => setImportPath(event.currentTarget.value)}
                placeholder="/Users/name/Downloads/wallpaper-backup.json"
                value={importPath}
              />
            </label>
            <button
              className="secondary-button"
              disabled={busy === "backup" || !importPath.trim()}
              type="submit"
            >
              Import backup
            </button>
          </form>
        </div>
      </section>

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
