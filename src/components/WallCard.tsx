import { convertFileSrc } from "@tauri-apps/api/core";
import { Download, Heart, MonitorUp } from "lucide-react";
import { useAppState } from "../appState";
import { FallbackImage } from "./FallbackImage";
import { Wallpaper } from "../types";

interface WallCardProps {
  wallpaper: Wallpaper;
}

export function WallCard({ wallpaper }: WallCardProps) {
  const { busy, favoriteIds, actions } = useAppState();
  const isSetting = busy === `set-${wallpaper.id}`;
  const isSaving = busy === `favorite-${wallpaper.id}`;
  const isSaved = favoriteIds.has(wallpaper.id);
  const image = wallpaper.localPath
    ? convertFileSrc(wallpaper.localPath)
    : wallpaper.thumbUrl || wallpaper.fullUrl || "";

  return (
    <article className="wall-card">
      <div className="wall-thumb">
        <FallbackImage
          alt={`${wallpaper.source} wallpaper by ${wallpaper.photographer}`}
          fallback={
          <div className="wall-thumb-empty">
            <Download size={28} aria-hidden="true" />
          </div>
          }
          src={image}
        />
      </div>
      <div className="wall-meta">
        <div>
          <strong>{wallpaper.source}</strong>
          <span>{wallpaper.photographer || "Unknown photographer"}</span>
        </div>
        <span>
          {wallpaper.width > 0 && wallpaper.height > 0
            ? `${wallpaper.width} x ${wallpaper.height}`
            : "cached"}
        </span>
      </div>
      <div className="wall-actions">
        <button
          className="icon-button"
          disabled={isSetting}
          onClick={() => actions.setWallpaper(wallpaper)}
          aria-label="Set as wallpaper"
          title="Set as wallpaper"
          type="button"
        >
          <MonitorUp size={17} aria-hidden="true" />
        </button>
        <button
          aria-label={isSaved ? "Saved favorite" : "Save favorite"}
          className={isSaved ? "icon-button saved" : "icon-button"}
          disabled={isSaving}
          onClick={() => actions.saveFavorite(wallpaper)}
          title={isSaved ? "Saved favorite" : "Save favorite"}
          type="button"
        >
          <Heart
            size={17}
            aria-hidden="true"
            fill={isSaved ? "currentColor" : "none"}
          />
        </button>
      </div>
    </article>
  );
}
