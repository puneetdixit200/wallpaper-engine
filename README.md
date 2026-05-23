# Wallpaper Engine

Small Tauri desktop app for finding, saving, caching, and applying still wallpapers from Pexels and Unsplash.

## Features

- Save Pexels and Unsplash API keys from the Settings screen.
- Search both providers, or use either provider alone.
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
