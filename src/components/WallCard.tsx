import { convertFileSrc } from "@tauri-apps/api/core";
import { Download, Heart, MonitorUp } from "lucide-react";
import { Wallpaper } from "../types";

interface WallCardProps {
  busy: string | null;
  wallpaper: Wallpaper;
  onSetWallpaper: (wallpaper: Wallpaper) => void;
  onSaveFavorite: (wallpaper: Wallpaper) => void;
}

export function WallCard({
  busy,
  wallpaper,
  onSetWallpaper,
  onSaveFavorite,
}: WallCardProps) {
  const isSetting = busy === `set-${wallpaper.id}`;
  const isSaving = busy === `favorite-${wallpaper.id}`;
  const image = wallpaper.localPath
    ? convertFileSrc(wallpaper.localPath)
    : wallpaper.thumbUrl || wallpaper.fullUrl || "";

  return (
    <article className="wall-card">
      <div className="wall-thumb">
        {image ? (
          <img alt={`${wallpaper.source} wallpaper by ${wallpaper.photographer}`} src={image} />
        ) : (
          <div className="wall-thumb-empty">
            <Download size={28} aria-hidden="true" />
          </div>
        )}
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
          onClick={() => onSetWallpaper(wallpaper)}
          title="Set as wallpaper"
          type="button"
        >
          <MonitorUp size={17} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          disabled={isSaving}
          onClick={() => onSaveFavorite(wallpaper)}
          title="Save favorite"
          type="button"
        >
          <Heart size={17} aria-hidden="true" />
        </button>
      </div>
    </article>
  );
}
