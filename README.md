# Wallpaper Engine

Small Tauri desktop app for finding, saving, caching, and applying still wallpapers from Pexels and Unsplash.

## Features

- Save Pexels, Unsplash, Pixabay, Wallhaven, and DeviantArt credentials from the Settings screen.
- Search all supported providers together, or use one provider alone.
- Use no-key providers for Wallhaven SFW search and Lorem Picsum placeholders.
- Enable Wallhaven sketchy/NSFW results with a Wallhaven API key.
- Switch between system, light, and dark themes.
- Apply a downloaded image as the desktop wallpaper.
- Save favorites and keep downloaded wallpapers in a local SQLite-backed cache.
- Auto-change wallpapers while the app is running.
- Build bundles for Windows, Linux, and macOS through GitHub Actions.

## Development

```bash
npm install
npm run tauri dev
```

## Verification

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

## API Keys

- Pexels: https://www.pexels.com/api/
- Unsplash: https://unsplash.com/developers
- Pixabay: https://pixabay.com/api/docs/
- Wallhaven: https://wallhaven.cc/help/api
- DeviantArt: https://www.deviantart.com/developers/

ArtStation does not expose a stable public search API. The app shows it as a source with a clear error instead of scraping unofficial endpoints.
