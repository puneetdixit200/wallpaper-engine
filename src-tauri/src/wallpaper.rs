use crate::settings::WallpaperLayoutPreference;
use image::{imageops::FilterType, GenericImageView, ImageFormat, ImageReader};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn set_desktop_wallpaper(path: &Path, layout: WallpaperLayoutPreference) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Wallpaper file does not exist: {}", path.display()));
    }

    set_platform_wallpaper(path, layout)
}

pub fn prepare_wallpaper_for_screen(path: &Path, cache_dir: &Path) -> PathBuf {
    let Some(screen_size) = current_screen_size() else {
        return path.to_path_buf();
    };

    resize_wallpaper_for_screen(path, &cache_dir.join("screen"), screen_size)
        .unwrap_or_else(|_| path.to_path_buf())
}

pub fn resize_wallpaper_for_screen(
    path: &Path,
    output_dir: &Path,
    screen_size: (u32, u32),
) -> Result<PathBuf, String> {
    let image = ImageReader::open(path)
        .map_err(|error| format!("Could not open wallpaper for screen sizing: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("Could not read wallpaper format: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode wallpaper for screen sizing: {error}"))?;
    let Some((next_width, next_height)) =
        fit_wallpaper_to_screen_dimensions(image.dimensions(), screen_size)
    else {
        return Ok(path.to_path_buf());
    };

    fs::create_dir_all(output_dir)
        .map_err(|error| format!("Could not create screen-sized wallpaper cache: {error}"))?;
    let target = output_dir.join(format!(
        "{}-{}x{}.jpg",
        safe_file_stem(
            path.file_stem()
                .map(|stem| stem.to_string_lossy())
                .unwrap_or_else(|| "wallpaper".into())
                .as_ref()
        ),
        next_width,
        next_height
    ));
    if target.exists() {
        return Ok(target);
    }

    let resized = image
        .resize_exact(next_width, next_height, FilterType::Lanczos3)
        .to_rgb8();
    image::DynamicImage::ImageRgb8(resized)
        .save_with_format(&target, ImageFormat::Jpeg)
        .map_err(|error| format!("Could not save screen-sized wallpaper: {error}"))?;

    Ok(target)
}

#[cfg(target_os = "windows")]
fn set_platform_wallpaper(path: &Path, layout: WallpaperLayoutPreference) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDWININICHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER,
    };

    set_windows_wallpaper_style(layout)?;

    let wide_path: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide_path.as_ptr() as *mut core::ffi::c_void,
            SPIF_UPDATEINIFILE | SPIF_SENDWININICHANGE,
        )
    };

    if result == 0 {
        Err("Windows rejected the wallpaper update.".into())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn set_platform_wallpaper(path: &Path, _layout: WallpaperLayoutPreference) -> Result<(), String> {
    let script = format!(
        "tell application \"System Events\" to set picture of every desktop to \"{}\"",
        escape_osascript_path(path)
    );
    command_status(
        Command::new("osascript").arg("-e").arg(script).status(),
        "osascript",
    )
}

#[cfg(target_os = "linux")]
fn set_platform_wallpaper(path: &Path, _layout: WallpaperLayoutPreference) -> Result<(), String> {
    let mut failures = Vec::new();

    for command in linux_wallpaper_commands(path) {
        if !command_exists(&command.program) {
            continue;
        }

        match Command::new(&command.program).args(&command.args).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => failures.push(format!("{} exited with {status}", command.program)),
            Err(error) => failures.push(format!("{} failed: {error}", command.program)),
        }
    }

    Err(if failures.is_empty() {
        "No supported Linux wallpaper tool was found. Install gsettings, swww, feh, or xwallpaper."
            .into()
    } else {
        failures.join("; ")
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn set_platform_wallpaper(_path: &Path, _layout: WallpaperLayoutPreference) -> Result<(), String> {
    Err("Wallpaper changes are not supported on this operating system.".into())
}

pub fn linux_wallpaper_commands(path: &Path) -> Vec<WallpaperCommand> {
    let path_text = path.to_string_lossy().to_string();
    let uri = file_uri(path);
    vec![
        WallpaperCommand {
            program: "gsettings".into(),
            args: vec![
                "set".into(),
                "org.gnome.desktop.background".into(),
                "picture-uri".into(),
                uri.clone(),
            ],
        },
        WallpaperCommand {
            program: "gsettings".into(),
            args: vec![
                "set".into(),
                "org.gnome.desktop.background".into(),
                "picture-uri-dark".into(),
                uri,
            ],
        },
        WallpaperCommand {
            program: "swww".into(),
            args: vec!["img".into(), path_text.clone()],
        },
        WallpaperCommand {
            program: "feh".into(),
            args: vec!["--bg-fill".into(), path_text.clone()],
        },
        WallpaperCommand {
            program: "xwallpaper".into(),
            args: vec!["--zoom".into(), path_text],
        },
    ]
}

pub fn windows_layout_registry_values(
    layout: WallpaperLayoutPreference,
) -> [(&'static str, &'static str); 2] {
    let (style, tile) = match layout {
        WallpaperLayoutPreference::Fill => ("10", "0"),
        WallpaperLayoutPreference::Fit => ("6", "0"),
        WallpaperLayoutPreference::Stretch => ("2", "0"),
        WallpaperLayoutPreference::Tile => ("0", "1"),
        WallpaperLayoutPreference::Center => ("0", "0"),
        WallpaperLayoutPreference::Span => ("22", "0"),
    };

    [("WallpaperStyle", style), ("TileWallpaper", tile)]
}

pub fn fit_wallpaper_to_screen_dimensions(
    image_size: (u32, u32),
    screen_size: (u32, u32),
) -> Option<(u32, u32)> {
    let (image_width, image_height) = image_size;
    let (screen_width, screen_height) = screen_size;
    if image_width == 0
        || image_height == 0
        || screen_width == 0
        || screen_height == 0
        || (image_width <= screen_width && image_height <= screen_height)
    {
        return None;
    }

    let scale =
        (screen_width as f64 / image_width as f64).min(screen_height as f64 / image_height as f64);
    let next_width = (image_width as f64 * scale).round().max(1.0) as u32;
    let next_height = (image_height as f64 * scale).round().max(1.0) as u32;

    Some((next_width, next_height))
}

#[cfg(target_os = "windows")]
fn current_screen_size() -> Option<(u32, u32)> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if width > 0 && height > 0 {
        Some((width as u32, height as u32))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn current_screen_size() -> Option<(u32, u32)> {
    None
}

fn safe_file_stem(value: &str) -> String {
    let safe: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();

    if safe.is_empty() {
        "wallpaper".into()
    } else {
        safe
    }
}

#[cfg(target_os = "windows")]
fn set_windows_wallpaper_style(layout: WallpaperLayoutPreference) -> Result<(), String> {
    for (name, value) in windows_layout_registry_values(layout) {
        let status = Command::new("reg")
            .args([
                "add",
                r"HKCU\Control Panel\Desktop",
                "/v",
                name,
                "/t",
                "REG_SZ",
                "/d",
                value,
                "/f",
            ])
            .status()
            .map_err(|error| format!("Could not set Windows wallpaper fit style: {error}"))?;

        if !status.success() {
            return Err(format!(
                "Windows wallpaper fit style update exited with {status}"
            ));
        }
    }

    Ok(())
}

fn file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", encode_uri_path(&normalized))
}

fn encode_uri_path(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(target_os = "macos")]
fn escape_osascript_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn command_status(
    status: std::io::Result<std::process::ExitStatus>,
    program: &str,
) -> Result<(), String> {
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("{program} exited with {status}")),
        Err(error) => Err(format!("{program} failed: {error}")),
    }
}

#[cfg(target_os = "linux")]
fn command_exists(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|path| path.join(program))
                .any(|path| path.exists())
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("wallpaper-engine-{name}-{nanos}"))
    }

    #[test]
    fn linux_commands_include_gnome_and_common_wallpaper_tools() {
        let commands = linux_wallpaper_commands(Path::new("/home/me/Pictures/wall one.jpg"));

        assert!(commands.iter().any(|command| command.program == "gsettings"
            && command.args.contains(&"picture-uri".to_string())
            && command
                .args
                .contains(&"file:///home/me/Pictures/wall%20one.jpg".to_string())));
        assert!(commands.iter().any(|command| command.program == "swww"));
        assert!(commands.iter().any(|command| command.program == "feh"));
        assert!(commands
            .iter()
            .any(|command| command.program == "xwallpaper"));
    }

    #[test]
    fn windows_layouts_use_windows_personalization_registry_values() {
        let cases = [
            (WallpaperLayoutPreference::Fill, "10", "0"),
            (WallpaperLayoutPreference::Fit, "6", "0"),
            (WallpaperLayoutPreference::Stretch, "2", "0"),
            (WallpaperLayoutPreference::Tile, "0", "1"),
            (WallpaperLayoutPreference::Center, "0", "0"),
            (WallpaperLayoutPreference::Span, "22", "0"),
        ];

        for (layout, style, tile) in cases {
            assert_eq!(
                windows_layout_registry_values(layout),
                [("WallpaperStyle", style), ("TileWallpaper", tile)]
            );
        }
    }

    #[test]
    fn oversized_wallpapers_are_sized_inside_screen_without_upscaling() {
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((3840, 2160), (1920, 1080)),
            Some((1920, 1080))
        );
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((4000, 3000), (1920, 1080)),
            Some((1440, 1080))
        );
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((1280, 720), (1920, 1080)),
            None
        );
    }

    #[test]
    fn resizes_oversized_image_for_screen_cache() {
        let dir = temp_dir("screen-resize");
        let output_dir = dir.join("screen");
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        let source = dir.join("source.jpg");
        image::RgbImage::new(400, 200)
            .save(&source)
            .expect("source image should save");

        let resized =
            resize_wallpaper_for_screen(&source, &output_dir, (200, 100)).expect("should resize");
        let resized_image = image::open(&resized).expect("resized image should open");

        assert_ne!(resized, source);
        assert_eq!(resized_image.dimensions(), (200, 100));

        let _ = std::fs::remove_dir_all(dir);
    }
}
