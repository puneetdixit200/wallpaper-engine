# Wallpaper Engine MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a small Tauri v2 desktop wallpaper app for Windows, Linux, and macOS with saved Pexels and Unsplash keys, search, favorites, cache, wallpaper setting, and auto-change while the app is running.

**Architecture:** React renders the four app views and calls Tauri commands. Rust owns settings, API requests, SQLite metadata, cache files, OS wallpaper changes, and the scheduler task. GitHub Actions builds release artifacts on Windows, Linux, and macOS.

**Tech Stack:** Tauri v2, React, TypeScript, Vite, Rust, reqwest, rusqlite, tokio, serde.

---

## File Map

- `src-tauri/src/settings.rs`: load and save API keys, timer, resolution, and cache settings.
- `src-tauri/src/models.rs`: shared serializable data shapes.
- `src-tauri/src/api.rs`: Pexels and Unsplash fetchers plus merged fallback fetch.
- `src-tauri/src/cache.rs`: SQLite table, downloads, cache stats, favorites, and library queries.
- `src-tauri/src/wallpaper.rs`: Windows, macOS, and Linux wallpaper setters.
- `src-tauri/src/scheduler.rs`: app-running auto-change task.
- `src-tauri/src/lib.rs`: Tauri state and command wiring.
- `src/App.tsx`: full app UI with Home, Search, Library, and Settings views.
- `src/App.css`: responsive desktop app styling.
- `.github/workflows/build.yml`: cross-platform build matrix.

## Tasks

### Task 1: Scaffold and Verify Baseline

- [x] Create the Tauri v2 React TypeScript scaffold with `npm create tauri-app@latest`.
- [x] Install npm dependencies with `npm install`.
- [ ] Run `npm run build`; expected result is a clean frontend build.
- [ ] Commit with `chore: scaffold tauri wallpaper app`.

### Task 2: Settings Persistence

- [ ] Add failing Rust tests in `src-tauri/src/settings.rs` for saving and loading Pexels and Unsplash keys from a JSON file.
- [ ] Run `cargo test settings`; expected failure is missing settings implementation.
- [ ] Implement `AppSettings`, `ApiKeys`, `load_settings_from_path`, and `save_settings_to_path`.
- [ ] Run `cargo test settings`; expected result is pass.
- [ ] Add Tauri commands `get_settings` and `save_settings`.
- [ ] Commit with `feat: persist wallpaper app settings`.

### Task 3: API and Cache Core

- [ ] Add Rust tests for mapping Pexels and Unsplash JSON into the shared `Wallpaper` model.
- [ ] Implement API fetchers with missing-key fallback: Both tries every configured source, and a single configured source still works if the other key is empty.
- [ ] Add SQLite setup and tests for inserting favorites and listing cached wallpapers.
- [ ] Implement image downloads into the app cache directory.
- [ ] Commit with `feat: add wallpaper search and cache core`.

### Task 4: OS Wallpaper Commands

- [ ] Add tests around shell-command selection for Linux and path handling for platform setters where practical.
- [ ] Implement Windows `SystemParametersInfoW`, macOS `osascript`, and Linux fallbacks for `gsettings`, `swww`, `feh`, and `xwallpaper`.
- [ ] Expose `set_wallpaper`, `save_favorite`, `list_library`, `cache_stats`, and `clear_cache` commands.
- [ ] Commit with `feat: wire wallpaper commands`.

### Task 5: React App UI

- [ ] Replace the starter React page with Home, Search, Library, and Settings tabs.
- [ ] Add key entry boxes for Pexels and Unsplash in Settings and save through the Rust command.
- [ ] Add search results, wallpaper cards, favorites, cache controls, and timer controls.
- [ ] Run `npm run build`; expected result is clean TypeScript and Vite output.
- [ ] Commit with `feat: build wallpaper app interface`.

### Task 6: Scheduler and Cross-Platform Builds

- [ ] Implement an app-running scheduler command that starts, stops, and updates the auto-change interval from settings.
- [ ] Add `.github/workflows/build.yml` with Windows, Linux, and macOS matrix jobs that run frontend build, Rust tests, and `npm run tauri build`.
- [ ] Run local `npm run build`, `cargo test`, and `npm run tauri build` on Windows.
- [ ] Push all commits to `https://github.com/puneetdixit200/wallpaper-engine.git`.
