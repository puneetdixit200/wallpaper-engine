import { WallCard } from "../components/WallCard";
import { Library, Wallpaper } from "../types";

interface LibraryPageProps {
  busy: string | null;
  library: Library;
  onSetWallpaper: (wallpaper: Wallpaper) => void;
  onSaveFavorite: (wallpaper: Wallpaper) => void;
}

export function LibraryPage({
  busy,
  library,
  onSetWallpaper,
  onSaveFavorite,
}: LibraryPageProps) {
  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Library</p>
          <h2>Saved and downloaded wallpapers</h2>
        </div>
      </header>

      <section className="section-band">
        <div className="section-title">
          <h3>Saved walls</h3>
          <span>{library.favorites.length}</span>
        </div>
        <div className="wall-grid">
          {library.favorites.map((wallpaper) => (
            <WallCard
              busy={busy}
              key={wallpaper.id}
              onSaveFavorite={onSaveFavorite}
              onSetWallpaper={onSetWallpaper}
              wallpaper={wallpaper}
            />
          ))}
        </div>
      </section>

      <section className="section-band">
        <div className="section-title">
          <h3>Downloaded</h3>
          <span>{library.downloaded.length}</span>
        </div>
        <div className="wall-grid">
          {library.downloaded.map((wallpaper) => (
            <WallCard
              busy={busy}
              key={wallpaper.id}
              onSaveFavorite={onSaveFavorite}
              onSetWallpaper={onSetWallpaper}
              wallpaper={wallpaper}
            />
          ))}
        </div>
      </section>
    </div>
  );
}
