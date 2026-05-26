import { convertFileSrc } from "@tauri-apps/api/core";
import { Heart, Shuffle, SkipForward } from "lucide-react";
import { MoodBar } from "../components/MoodBar";
import { Mood, trendingTopics, Wallpaper } from "../types";

interface HomePageProps {
  busy: string | null;
  currentWallpaper: Wallpaper | null;
  mood: Mood;
  notice: string;
  providerState: string;
  onMoodSelect: (mood: Mood) => void;
  onNext: () => void;
  onRandom: () => void;
  onSaveCurrent: () => void | Promise<void> | undefined;
  onTopicSelect: (query: string) => void | Promise<void>;
}

export function HomePage({
  busy,
  currentWallpaper,
  mood,
  notice,
  providerState,
  onMoodSelect,
  onNext,
  onRandom,
  onSaveCurrent,
  onTopicSelect,
}: HomePageProps) {
  const preview = currentWallpaper?.localPath
    ? convertFileSrc(currentWallpaper.localPath)
    : currentWallpaper?.fullUrl || currentWallpaper?.thumbUrl || "";

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
        {preview ? (
          <img alt="Current wallpaper preview" src={preview} />
        ) : (
          <div className="preview-empty">
            <span>Wallpaper preview</span>
          </div>
        )}
        <div className="preview-overlay">
          <div>
            <strong>{currentWallpaper?.source || "No wallpaper applied yet"}</strong>
            <span>{currentWallpaper?.photographer || "Choose random, next, or search"}</span>
          </div>
          <div className="action-row">
            <button
              className="primary-button"
              disabled={busy === "random"}
              onClick={onRandom}
              type="button"
            >
              <Shuffle size={17} aria-hidden="true" />
              Random
            </button>
            <button
              className="secondary-button"
              disabled={busy === "next"}
              onClick={onNext}
              type="button"
            >
              <SkipForward size={17} aria-hidden="true" />
              Next
            </button>
            <button
              className="secondary-button"
              disabled={!currentWallpaper}
              onClick={onSaveCurrent}
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
        <MoodBar activeMood={mood} onMoodSelect={onMoodSelect} />
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
              onClick={() => onTopicSelect(topic.query)}
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
