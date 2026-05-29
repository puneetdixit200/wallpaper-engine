import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Download,
  Eye,
  Heart,
  ListPlus,
  Lock,
  MonitorUp,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useAppState } from "../appState";
import { FallbackImage } from "./FallbackImage";
import { Wallpaper, WallpaperLayoutPreference } from "../types";
import {
  objectFitForLayout,
  wallpaperQualityWarnings,
} from "../wallpaperQuality";

interface WallCardProps {
  wallpaper: Wallpaper;
  canDelete?: boolean;
}

export function WallCard({ wallpaper, canDelete = false }: WallCardProps) {
  const { busy, favoriteIds, library, settings, actions } = useAppState();
  const [isPreviewOpen, setIsPreviewOpen] = useState(false);
  const [previewLayout, setPreviewLayout] =
    useState<WallpaperLayoutPreference>(settings.wallpaperLayout);
  const [playlistId, setPlaylistId] = useState(
    () => library.playlists[0]?.id ?? "",
  );
  const isSetting = busy === `set-${wallpaper.id}`;
  const isSaving = busy === `favorite-${wallpaper.id}`;
  const isDeleting = busy === `delete-${wallpaper.id}`;
  const isLocking = busy === `lock-${wallpaper.id}`;
  const isAddingToPlaylist = busy === `playlist-${wallpaper.id}`;
  const isSaved = favoriteIds.has(wallpaper.id);
  const qualityWarnings = useMemo(
    () => wallpaperQualityWarnings(wallpaper, settings),
    [settings, wallpaper],
  );
  const image = wallpaper.localPath
    ? convertFileSrc(wallpaper.localPath)
    : wallpaper.thumbUrl || wallpaper.fullUrl || "";
  const hasPlaylists = library.playlists.length > 0;

  useEffect(() => {
    setPreviewLayout(settings.wallpaperLayout);
  }, [settings.wallpaperLayout]);

  useEffect(() => {
    if (playlistId && library.playlists.some((playlist) => playlist.id === playlistId)) {
      return;
    }
    setPlaylistId(library.playlists[0]?.id ?? "");
  }, [library.playlists, playlistId]);

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
      {qualityWarnings.length > 0 ? (
        <div className="wall-badges">
          <span>Quality check</span>
        </div>
      ) : null}
      <div className="wall-actions">
        <button
          aria-label="Preview crop and fit"
          className="icon-button"
          onClick={() => setIsPreviewOpen(true)}
          title="Preview crop and fit"
          type="button"
        >
          <Eye size={17} aria-hidden="true" />
        </button>
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
          aria-label="Set lock screen wallpaper"
          className="icon-button"
          disabled={isLocking}
          onClick={() => actions.setLockScreenWallpaper(wallpaper)}
          title="Set lock screen wallpaper"
          type="button"
        >
          <Lock size={17} aria-hidden="true" />
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
        {canDelete ? (
          <button
            aria-label="Delete wallpaper"
            className="icon-button danger"
            disabled={isDeleting}
            onClick={() => void actions.deleteWallpaper(wallpaper)}
            title="Delete wallpaper"
            type="button"
          >
            <Trash2 size={17} aria-hidden="true" />
          </button>
        ) : null}
      </div>
      {hasPlaylists ? (
        <div className="playlist-picker">
          <select
            aria-label="Choose playlist"
            onChange={(event) => setPlaylistId(event.currentTarget.value)}
            value={playlistId}
          >
            {library.playlists.map((playlist) => (
              <option key={playlist.id} value={playlist.id}>
                {playlist.name}
              </option>
            ))}
          </select>
          <button
            aria-label="Add to playlist"
            className="icon-button"
            disabled={!playlistId || isAddingToPlaylist}
            onClick={() => actions.addWallpaperToPlaylist(playlistId, wallpaper)}
            title="Add to playlist"
            type="button"
          >
            <ListPlus size={17} aria-hidden="true" />
          </button>
        </div>
      ) : null}
      {isPreviewOpen ? (
        <div
          aria-label="Wallpaper preview"
          aria-modal="true"
          className="modal-scrim"
          role="dialog"
          onClick={() => setIsPreviewOpen(false)}
        >
          <section
            className="preview-modal"
            onClick={(event) => event.stopPropagation()}
          >
            <header className="modal-header">
              <div>
                <p className="eyebrow">Preview</p>
                <h3>Crop and fit</h3>
              </div>
              <button
                aria-label="Close preview"
                className="icon-button"
                onClick={() => setIsPreviewOpen(false)}
                type="button"
              >
                <X size={18} aria-hidden="true" />
              </button>
            </header>

            <div className="crop-preview-frame">
              <FallbackImage
                alt={`${wallpaper.source} wallpaper preview`}
                fallback={
                  <div className="wall-thumb-empty">
                    <Download size={28} aria-hidden="true" />
                  </div>
                }
                src={image}
                style={{ objectFit: objectFitForLayout(previewLayout) }}
              />
            </div>

            <div className="layout-selector" aria-label="Wallpaper layout">
              {(["fill", "fit", "stretch", "tile", "center", "span"] as const).map(
                (layout) => (
                  <button
                    aria-pressed={previewLayout === layout}
                    className={previewLayout === layout ? "active" : ""}
                    key={layout}
                    onClick={() => setPreviewLayout(layout)}
                    type="button"
                  >
                    {layout}
                  </button>
                ),
              )}
            </div>

            {qualityWarnings.length > 0 ? (
              <ul className="quality-list">
                {qualityWarnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            ) : null}

            <div className="modal-actions">
              <button
                className="primary-button"
                disabled={isSetting}
                onClick={() => {
                  void actions.setWallpaperWithLayout(wallpaper, previewLayout);
                  setIsPreviewOpen(false);
                }}
                type="button"
              >
                Apply layout
              </button>
              <button
                className="secondary-button"
                onClick={() => setIsPreviewOpen(false)}
                type="button"
              >
                Close
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </article>
  );
}
