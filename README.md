# Wallpaper Engine - Automatic Desktop Wallpaper Changer

[![Build desktop app](https://github.com/puneetdixit200/wallpaper-engine/actions/workflows/build.yml/badge.svg)](https://github.com/puneetdixit200/wallpaper-engine/actions/workflows/build.yml)

Wallpaper Engine is an open-source desktop wallpaper changer for Windows, macOS, and Linux. It helps you search wallpaper providers, download high-resolution images, save favorites, manage a local wallpaper library, and automatically rotate wallpapers in the background after the main window is closed.

Use it as a lightweight Tauri wallpaper manager for Pexels, Unsplash, Pixabay, Wallhaven, DeviantArt, local cache management, tray background mode, startup wallpaper updates, and one-click desktop wallpaper application.

## Quick Links

- Website: https://puneetdixit200.github.io/wallpaper-engine/
- Source: https://github.com/puneetdixit200/wallpaper-engine
- Installers: [`installers/`](installers/)
- Checksums: [`installers/SHA256SUMS`](installers/SHA256SUMS)

## One-Click Downloads

Click a link below to download the current packaged installer directly from this repo. The same files are committed under `installers/` for source checkouts.

| Platform | Direct download | Use when |
| --- | --- | --- |
| Windows setup EXE | [Download EXE](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/windows/Wallpaper-Engine_0.1.0_x64-setup.exe) | Standard Windows install |
| Windows MSI | [Download MSI](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/windows/Wallpaper-Engine_0.1.0_x64_en-US.msi) | Managed/manual MSI install |
| Windows uninstaller EXE | [Download Uninstaller](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/windows/Wallpaper-Engine_0.1.0_x64-uninstaller.exe) | Remove the app, background process, startup entry, and app cache |
| macOS Apple Silicon | [Download DMG](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/macos/Wallpaper-Engine_0.1.0_aarch64.dmg) | M1/M2/M3/M4 Macs |
| Linux AppImage | [Download AppImage](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/linux/Wallpaper-Engine_0.1.0_amd64.AppImage) | Portable Linux run without package install |
| Linux DEB | [Download DEB](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/linux/Wallpaper-Engine_0.1.0_amd64.deb) | Debian, Ubuntu, Linux Mint |
| Linux RPM | [Download RPM](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/linux/Wallpaper-Engine-0.1.0-1.x86_64.rpm) | Fedora, RHEL, openSUSE-style RPM systems |
| Checksums | [Download SHA256SUMS](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/SHA256SUMS) | Verify installer integrity |

Verify downloaded files:

```bash
shasum -a 256 -c installers/SHA256SUMS
```

On Linux, `sha256sum -c installers/SHA256SUMS` also works.

## Why Use Wallpaper Engine

- Search multiple wallpaper sources from one desktop app instead of opening separate sites.
- Keep original high-resolution files in the local cache so wallpapers stay sharp before they are set.
- Save favorites, delete individual wallpapers, or clear the full local wallpaper library from the app.
- Run background wallpaper rotation from the tray after closing the window.
- Enable startup behavior so wallpaper auto-change resumes after login.
- Use committed installers for Windows, macOS, and Linux with checksum verification.

## Features

- Search Pexels, Unsplash, Pixabay, Wallhaven, DeviantArt, and fallback no-key providers.
- Save provider credentials from the Settings screen.
- Enable Wallhaven sketchy/NSFW results with a Wallhaven API key.
- Filter provider results to desktop-sized landscape wallpapers.
- Save favorites and downloaded wallpapers in a local SQLite-backed cache.
- Apply downloaded wallpapers as the desktop background.
- Apply wallpapers with Fill, Fit, Stretch, Tile, Center, or Span layout on Windows and compatible Linux desktops.
- Preserve original high-resolution files for mismatched aspect-ratio wallpapers so Fill layout does not upscale a pre-shrunk image.
- Clear the library and remove downloaded wallpaper files from storage.
- Delete wallpapers one by one from the Library screen.
- Keep the left sidebar static on desktop while content scrolls.
- Use an adaptive UI that responds to app window resizing.
- Ask before enabling background mode.
- Keep the saved auto-change interval running from the tray after the window is closed.
- Launch hidden on startup when background mode is enabled.
- Keep the background service alive on close or OS-level quit.
- Use the tray Quit action, or terminate the process manually, to stop background updates.
- Minimize to tray on Windows so wallpaper updates continue in the background.
- Build bundles for Windows, Linux, and macOS through GitHub Actions.

## Install

### Windows

Download or clone the repo, then run the setup installer:

```powershell
.\installers\windows\Wallpaper-Engine_0.1.0_x64-setup.exe
```

For MSI-based installs:

```powershell
msiexec /i .\installers\windows\Wallpaper-Engine_0.1.0_x64_en-US.msi
```

The setup EXE and MSI register Wallpaper Engine in Windows Installed Apps.
For direct cleanup, run:

```powershell
.\installers\windows\Wallpaper-Engine_0.1.0_x64-uninstaller.exe
```

The standalone uninstaller stops the app process, runs the registered Windows uninstaller when present, removes startup entries, and deletes app-specific data/cache folders.

### macOS

Download or clone the repo, then open the DMG:

```bash
open installers/macos/Wallpaper-Engine_0.1.0_aarch64.dmg
```

Drag `Wallpaper Engine.app` to `Applications`, then launch it from Applications.

### Linux

Use one of the Linux packages from `installers/linux`.

AppImage:

```bash
chmod +x installers/linux/Wallpaper-Engine_0.1.0_amd64.AppImage
./installers/linux/Wallpaper-Engine_0.1.0_amd64.AppImage
```

Debian, Ubuntu, or Linux Mint:

```bash
sudo apt install ./installers/linux/Wallpaper-Engine_0.1.0_amd64.deb
wallpaper-engine
```

Fedora or RPM-based Linux:

```bash
sudo dnf install ./installers/linux/Wallpaper-Engine-0.1.0-1.x86_64.rpm
wallpaper-engine
```

Linux wallpaper application is ready for common desktop environments. GNOME-compatible desktops use `gsettings`; other sessions can use supported wallpaper tools such as `feh`.

## API Keys

- Pexels: https://www.pexels.com/api/
- Unsplash: https://unsplash.com/developers
- Pixabay: https://pixabay.com/api/docs/
- Wallhaven: https://wallhaven.cc/help/api
- DeviantArt: https://www.deviantart.com/developers/

ArtStation does not expose a stable public search API. The app shows it as a source with a clear error instead of scraping unofficial endpoints.

## Development

```bash
npm install
npm run tauri dev
```

## Verification

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path tools/windows-uninstaller/Cargo.toml
npm run tauri build
```

