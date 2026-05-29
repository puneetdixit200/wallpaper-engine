import { useEffect, useRef } from "react";
import { Filter, Search as SearchIcon } from "lucide-react";
import { useAppState } from "../appState";
import { EmptyState } from "../components/EmptyState";
import { WallCard } from "../components/WallCard";
import { WallGridSkeleton } from "../components/WallGridSkeleton";
import { buildProviderQuery, filterWallpapers } from "../searchFilters";
import { ApiSource, SearchOrientationFilter } from "../types";

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

const orientationOptions: Array<{
  label: string;
  value: SearchOrientationFilter;
}> = [
  { label: "Any", value: "any" },
  { label: "Landscape", value: "landscape" },
  { label: "Portrait", value: "portrait" },
  { label: "Square", value: "square" },
];

export function SearchPage() {
  const { busy, page, query, results, settings, source, actions } =
    useAppState();
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const requestedPageRef = useRef(page);
  const filters = settings.searchFilters;
  const filteredResults = filterWallpapers(results, filters);
  const hasRawResults = results.length > 0;
  const hasResults = filteredResults.length > 0;
  const isSearchLoading = busy === "search";
  const providerQuery = buildProviderQuery(query, filters);

  useEffect(() => {
    requestedPageRef.current = page;
  }, [filters, page, query, source]);

  useEffect(() => {
    if (
      !hasRawResults ||
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
        void actions.searchWallpapers(nextPage, providerQuery, source);
      },
      { rootMargin: "360px 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [actions, hasRawResults, isSearchLoading, page, providerQuery, source]);

  function updateFilters(nextFilters: Partial<typeof filters>) {
    void actions.saveSettings({
      ...settings,
      searchFilters: {
        ...filters,
        ...nextFilters,
      },
    });
  }

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
          void actions.searchWallpapers(1, providerQuery, source);
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

      <section aria-label="Search filters" className="filter-panel">
        <div className="filter-heading">
          <Filter size={18} aria-hidden="true" />
          <span>Provider filters</span>
        </div>
        <label>
          <span>Orientation</span>
          <select
            onChange={(event) =>
              updateFilters({
                orientation: event.currentTarget
                  .value as SearchOrientationFilter,
              })
            }
            value={filters.orientation}
          >
            {orientationOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Minimum width</span>
          <input
            min={0}
            max={15360}
            onChange={(event) =>
              updateFilters({
                minWidth: Number.parseInt(event.currentTarget.value, 10) || 0,
              })
            }
            step={100}
            type="number"
            value={filters.minWidth}
          />
        </label>
        <label>
          <span>Minimum height</span>
          <input
            min={0}
            max={8640}
            onChange={(event) =>
              updateFilters({
                minHeight: Number.parseInt(event.currentTarget.value, 10) || 0,
              })
            }
            step={100}
            type="number"
            value={filters.minHeight}
          />
        </label>
        <label>
          <span>Color hint</span>
          <input
            onChange={(event) => updateFilters({ color: event.currentTarget.value })}
            placeholder="blue, black, pastel..."
            value={filters.color}
          />
        </label>
      </section>

      <section className="wall-grid">
        {!hasRawResults && !isSearchLoading ? (
          <EmptyState
            title="No results yet"
            detail="Results will appear here."
          />
        ) : null}
        {hasRawResults && !hasResults && !isSearchLoading ? (
          <EmptyState
            title="No results match filters"
            detail="Try lowering the minimum size or changing orientation."
          />
        ) : null}
        {filteredResults.map((wallpaper) => (
          <WallCard key={wallpaper.id} wallpaper={wallpaper} />
        ))}
        {isSearchLoading ? (
          <WallGridSkeleton count={hasRawResults ? 3 : 6} />
        ) : null}
      </section>

      <div className="load-row" ref={sentinelRef}>
        <span>
          {hasRawResults
            ? isSearchLoading
              ? "Loading more wallpapers"
              : `${filteredResults.length} shown from page ${page}`
            : "No results loaded"}
        </span>
      </div>
    </div>
  );
}
