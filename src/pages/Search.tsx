import { Search as SearchIcon } from "lucide-react";
import { WallCard } from "../components/WallCard";
import { ApiSource, Wallpaper } from "../types";

interface SearchPageProps {
  busy: string | null;
  page: number;
  query: string;
  results: Wallpaper[];
  source: ApiSource;
  onLoadMore: () => void;
  onQueryChange: (query: string) => void;
  onSearch: () => void;
  onSetWallpaper: (wallpaper: Wallpaper) => void;
  onSaveFavorite: (wallpaper: Wallpaper) => void;
  onSourceChange: (source: ApiSource) => void;
}

const sourceOptions: Array<{ label: string; value: ApiSource }> = [
  { label: "all", value: "all" },
  { label: "pexels", value: "pexels" },
  { label: "unsplash", value: "unsplash" },
  { label: "pixabay", value: "pixabay" },
  { label: "wallhaven", value: "wallhaven" },
  { label: "picsum", value: "picsum" },
  { label: "deviantArt", value: "deviantArt" },
  { label: "artStation", value: "artStation" },
];

export function SearchPage({
  busy,
  page,
  query,
  results,
  source,
  onLoadMore,
  onQueryChange,
  onSearch,
  onSetWallpaper,
  onSaveFavorite,
  onSourceChange,
}: SearchPageProps) {
  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Search</p>
          <h2>Find still wallpapers</h2>
        </div>
        <div className="segmented-control" aria-label="API source">
          {sourceOptions.map((option) => (
            <button
              className={source === option.value ? "active" : ""}
              key={option.value}
              onClick={() => onSourceChange(option.value)}
              type="button"
            >
              {option.label}
            </button>
          ))}
        </div>
      </header>

      <form
        className="search-bar"
        onSubmit={(event) => {
          event.preventDefault();
          onSearch();
        }}
      >
        <SearchIcon size={19} aria-hidden="true" />
        <input
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          placeholder="forest, city night, clean minimal..."
          value={query}
        />
        <button className="primary-button" disabled={busy === "search"} type="submit">
          Search
        </button>
      </form>

      <section className="wall-grid">
        {results.map((wallpaper) => (
          <WallCard
            busy={busy}
            key={wallpaper.id}
            onSaveFavorite={onSaveFavorite}
            onSetWallpaper={onSetWallpaper}
            wallpaper={wallpaper}
          />
        ))}
      </section>

      <div className="load-row">
        <span>{results.length ? `Page ${page} loaded` : "No results loaded"}</span>
        <button
          className="secondary-button"
          disabled={!results.length || busy === "search"}
          onClick={onLoadMore}
          type="button"
        >
          Load more
        </button>
      </div>
    </div>
  );
}
