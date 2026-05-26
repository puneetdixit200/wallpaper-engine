import { useEffect, useRef } from "react";
import { Search as SearchIcon } from "lucide-react";
import { useAppState } from "../appState";
import { EmptyState } from "../components/EmptyState";
import { WallCard } from "../components/WallCard";
import { WallGridSkeleton } from "../components/WallGridSkeleton";
import { ApiSource } from "../types";

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

export function SearchPage() {
  const { busy, page, query, results, source, actions } = useAppState();
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const requestedPageRef = useRef(page);
  const hasResults = results.length > 0;
  const isSearchLoading = busy === "search";

  useEffect(() => {
    requestedPageRef.current = page;
  }, [page, query, source]);

  useEffect(() => {
    if (
      !hasResults ||
      isSearchLoading ||
      typeof IntersectionObserver === "undefined"
    ) {
      return;
    }

    const node = sentinelRef.current;
    if (!node) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) {
          return;
        }

        const nextPage = page + 1;
        if (requestedPageRef.current >= nextPage) {
          return;
        }

        requestedPageRef.current = nextPage;
        void actions.searchWallpapers(nextPage);
      },
      { rootMargin: "360px 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [actions, hasResults, isSearchLoading, page]);

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
              onClick={() => void actions.changeSource(option.value)}
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
          void actions.searchWallpapers(1);
        }}
      >
        <SearchIcon size={19} aria-hidden="true" />
        <input
          onChange={(event) => actions.setQuery(event.currentTarget.value)}
          placeholder="forest, city night, clean minimal..."
          value={query}
        />
        <button className="primary-button" disabled={busy === "search"} type="submit">
          Search
        </button>
      </form>

      <section className="wall-grid">
        {!hasResults && !isSearchLoading ? (
          <EmptyState
            title="No results yet"
            detail="Results will appear here."
          />
        ) : null}
        {results.map((wallpaper) => (
          <WallCard key={wallpaper.id} wallpaper={wallpaper} />
        ))}
        {isSearchLoading ? <WallGridSkeleton count={hasResults ? 3 : 6} /> : null}
      </section>

      <div className="load-row" ref={sentinelRef}>
        <span>
          {hasResults
            ? isSearchLoading
              ? "Loading more wallpapers"
              : `Page ${page} loaded`
            : "No results loaded"}
        </span>
      </div>
    </div>
  );
}
