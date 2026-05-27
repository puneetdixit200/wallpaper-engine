# Wallpaper Engine

Wallpaper Engine is a Tauri desktop app for finding, saving, caching, and applying still wallpapers from multiple wallpaper providers.

## One-Click Downloads

Click a link below to download the current packaged installer directly from this repo.
The same files are also committed under `installers/` for source checkouts.

| Platform | Direct download | Use when |
| --- | --- | --- |
| Windows setup EXE | [Download EXE](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/windows/Wallpaper-Engine_0.1.0_x64-setup.exe) | Standard Windows install |
| Windows MSI | [Download MSI](https://raw.githubusercontent.com/puneetdixit200/wallpaper-engine/main/installers/windows/Wallpaper-Engine_0.1.0_x64_en-US.msi) | Managed/manual MSI install |
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

## Windows Install

Download or clone the repo, then run the standard setup installer:

```powershell
.\installers\windows\Wallpaper-Engine_0.1.0_x64-setup.exe
```

For MSI-based installs:

```powershell
msiexec /i .\installers\windows\Wallpaper-Engine_0.1.0_x64_en-US.msi
```

## macOS Install

Download or clone the repo, then open the DMG:

```bash
open installers/macos/Wallpaper-Engine_0.1.0_aarch64.dmg
```

Drag `Wallpaper Engine.app` to `Applications`, then launch it from Applications.

## Linux Install

Use one of the Linux packages from `installers/linux`.

### AppImage

```bash
chmod +x installers/linux/Wallpaper-Engine_0.1.0_amd64.AppImage
./installers/linux/Wallpaper-Engine_0.1.0_amd64.AppImage
```

If your distro does not include FUSE support for AppImages, install the distro package for FUSE first, then run the AppImage again.

### Debian or Ubuntu

```bash
sudo apt install ./installers/linux/Wallpaper-Engine_0.1.0_amd64.deb
wallpaper-engine
```

### Fedora or RPM-Based Linux

```bash
sudo dnf install ./installers/linux/Wallpaper-Engine-0.1.0-1.x86_64.rpm
wallpaper-engine
```

Linux wallpaper application is ready for common desktop environments. GNOME-compatible desktops use `gsettings`; other sessions can use supported wallpaper tools such as `feh`.

## Features

- Save Pexels, Unsplash, Pixabay, Wallhaven, and DeviantArt credentials from the Settings screen.
- Search all supported providers together, or use one provider alone.
- Use no-key providers for Wallhaven SFW search and Lorem Picsum placeholders.
- Enable Wallhaven sketchy/NSFW results with a Wallhaven API key.
- Switch between system, light, and dark themes.
- Apply a downloaded image as the desktop wallpaper.
- Apply wallpapers with Fill, Fit, Stretch, Tile, Center, or Span layout on Windows and compatible Linux desktops.
- Preserve original high-resolution files for mismatched aspect-ratio wallpapers so Fill layout does not upscale a pre-shrunk image.
- Keep the left sidebar static on desktop while content scrolls.
- Keep the active wallpaper locked while the app runs so Windows slideshow/theme changes are overwritten.
- Filter provider results to desktop-sized landscape wallpapers.
- Save favorites and keep downloaded wallpapers in a local SQLite-backed cache.
- Start the saved auto-change interval on launch and keep it running from the tray after the window is closed.
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
