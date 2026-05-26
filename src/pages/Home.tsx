import { convertFileSrc } from "@tauri-apps/api/core";
import { Heart, Shuffle, SkipForward } from "lucide-react";
import { useAppState } from "../appState";
import { FallbackImage } from "../components/FallbackImage";
import { MoodBar } from "../components/MoodBar";
import { trendingTopics } from "../types";

export function HomePage() {
  const {
    busy,
    currentWallpaper,
    hasAnyKey,
    mood,
    notice,
    actions,
  } = useAppState();
  const preview = currentWallpaper?.localPath
    ? convertFileSrc(currentWallpaper.localPath)
    : currentWallpaper?.fullUrl || currentWallpaper?.thumbUrl || "";
  const providerState = hasAnyKey ? "API keys saved" : "Free sources ready";

  return (
    <div className="view-stack">
      <header className="view-header">
        <div>
          <p className="eyebrow">Current wallpaper</p>
          <h2>Make the desktop look right now</h2>
        </div>
        <div className="key-state ready">
          {providerState}
        </div>
      </header>

      <section className="preview-panel">
        <FallbackImage
          alt="Current wallpaper preview"
          fallback={
          <div className="preview-empty">
            <span>Wallpaper preview</span>
          </div>
          }
          src={preview}
        />
        <div className="preview-overlay">
          <div>
            <strong>{currentWallpaper?.source || "No wallpaper applied yet"}</strong>
            <span>{currentWallpaper?.photographer || "Choose random, next, or search"}</span>
          </div>
          <div className="action-row">
            <button
              className="primary-button"
              disabled={busy === "random"}
              onClick={actions.applyRandomWallpaper}
              type="button"
            >
              <Shuffle size={17} aria-hidden="true" />
              Random
            </button>
            <button
              className="secondary-button"
              disabled={busy === "next"}
              onClick={actions.applyNextFromMood}
              type="button"
            >
              <SkipForward size={17} aria-hidden="true" />
              Next
            </button>
            <button
              className="secondary-button"
              disabled={!currentWallpaper}
              onClick={() =>
                currentWallpaper
                  ? void actions.saveFavorite(currentWallpaper)
                  : undefined
              }
              type="button"
            >
              <Heart size={17} aria-hidden="true" />
              Save
            </button>
          </div>
        </div>
      </section>

      <section className="section-band">
        <div className="section-title">
          <h3>Moods</h3>
          <span>{mood}</span>
        </div>
        <MoodBar activeMood={mood} onMoodSelect={actions.applyMood} />
      </section>

      <section className="section-band">
        <div className="section-title">
          <h3>Trending</h3>
          <span>wallpapers</span>
        </div>
        <div className="topic-grid" aria-label="Trending wallpaper topics">
          {trendingTopics.map((topic) => (
            <button
              className="topic-chip"
              key={topic.query}
              onClick={() => actions.applyTopic(topic.query)}
              type="button"
            >
              {topic.label}
            </button>
          ))}
        </div>
      </section>

      {notice ? <p className="notice">{notice}</p> : null}
    </div>
  );
}
