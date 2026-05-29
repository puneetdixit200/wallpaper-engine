use crate::settings::{ResolutionPreference, WallpaperLayoutPreference};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperLock {
    pub path: PathBuf,
    pub layout: WallpaperLayoutPreference,
}

pub fn set_desktop_wallpaper(path: &Path, layout: WallpaperLayoutPreference) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Wallpaper file does not exist: {}", path.display()));
    }

    set_platform_wallpaper(path, layout)
}

pub fn set_lock_screen_wallpaper(
    path: &Path,
    layout: WallpaperLayoutPreference,
) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Wallpaper file does not exist: {}", path.display()));
    }

    set_platform_lock_screen_wallpaper(path, layout)
}

pub fn restore_locked_wallpaper_if_needed<F>(
    lock: &WallpaperLock,
    current_wallpaper: Option<PathBuf>,
    set_wallpaper: F,
) -> Result<bool, String>
where
    F: FnOnce(&Path, WallpaperLayoutPreference) -> Result<(), String>,
{
    if current_wallpaper
        .as_deref()
        .is_some_and(|current| wallpaper_paths_match(current, &lock.path))
    {
        return Ok(false);
    }

    set_wallpaper(&lock.path, lock.layout)?;
    Ok(true)
}

pub fn wallpaper_lock_from_current_desktop(
    current_wallpaper: Option<PathBuf>,
    layout: WallpaperLayoutPreference,
) -> Option<WallpaperLock> {
    current_wallpaper.map(|path| WallpaperLock { path, layout })
}

pub fn restore_startup_wallpaper_if_app_changed<F>(
    startup_wallpaper: Option<&WallpaperLock>,
    active_wallpaper_lock: Option<&WallpaperLock>,
    current_wallpaper: Option<PathBuf>,
    set_wallpaper: F,
) -> Result<bool, String>
where
    F: FnOnce(&Path, WallpaperLayoutPreference) -> Result<(), String>,
{
    let Some(startup_wallpaper) = startup_wallpaper else {
        return Ok(false);
    };
    let Some(active_wallpaper_lock) = active_wallpaper_lock else {
        return Ok(false);
    };

    if wallpaper_paths_match(&startup_wallpaper.path, &active_wallpaper_lock.path)
        || current_wallpaper
            .as_deref()
            .is_some_and(|current| wallpaper_paths_match(current, &startup_wallpaper.path))
    {
        return Ok(false);
    }

    set_wallpaper(&startup_wallpaper.path, startup_wallpaper.layout)?;
    Ok(true)
}

#[cfg(target_os = "windows")]
pub fn current_desktop_wallpaper() -> Result<Option<PathBuf>, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETDESKWALLPAPER,
    };

    let mut buffer = vec![0_u16; 32_768];
    let result = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
            0,
        )
    };

    if result == 0 {
        return Err("Windows rejected the wallpaper read.".into());
    }

    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    if length == 0 {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..length],
        ))))
    }
}

#[cfg(target_os = "macos")]
pub fn current_desktop_wallpaper() -> Result<Option<PathBuf>, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(macos_current_wallpaper_script())
        .output()
        .map_err(|error| format!("osascript failed: {error}"))?;
    if !output.status.success() {
        return Err(format!("osascript exited with {}", output.status));
    }

    Ok(parse_current_wallpaper_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(target_os = "linux")]
pub fn current_desktop_wallpaper() -> Result<Option<PathBuf>, String> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.background", "picture-uri"])
        .output()
        .map_err(|error| format!("gsettings failed: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_current_wallpaper_output(
        raw.trim()
            .trim_matches('\'')
            .strip_prefix("file://")
            .unwrap_or(raw.trim()),
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn current_desktop_wallpaper() -> Result<Option<PathBuf>, String> {
    Ok(None)
}

pub fn prepare_wallpaper_for_screen(
    path: &Path,
    cache_dir: &Path,
    resolution: ResolutionPreference,
) -> PathBuf {
    let Some(screen_size) = screen_size_or_resolution(current_screen_size(), resolution) else {
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
    let reader = ImageReader::open(path)
        .map_err(|error| format!("Could not open wallpaper for screen sizing: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("Could not read wallpaper format: {error}"))?;
    let source_format = screen_cache_format(path, reader.format());
    let image = reader
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
        "{}-{}x{}.{}",
        safe_file_stem(
            path.file_stem()
                .map(|stem| stem.to_string_lossy())
                .unwrap_or_else(|| "wallpaper".into())
                .as_ref()
        ),
        next_width,
        next_height,
        source_format.extension
    ));
    if target.exists() {
        return Ok(target);
    }

    let resized = image.resize_exact(next_width, next_height, FilterType::Lanczos3);
    let output = if source_format.format == ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(resized.to_rgb8())
    } else {
        resized
    };
    output
        .save_with_format(&target, source_format.format)
        .map_err(|error| format!("Could not save screen-sized wallpaper: {error}"))?;

    Ok(target)
}

#[derive(Clone, Copy)]
struct ScreenCacheFormat {
    extension: &'static str,
    format: ImageFormat,
}

fn screen_cache_format(path: &Path, guessed_format: Option<ImageFormat>) -> ScreenCacheFormat {
    guessed_format
        .and_then(screen_cache_format_from_image_format)
        .or_else(|| screen_cache_format_from_extension(path))
        .unwrap_or(ScreenCacheFormat {
            extension: "jpg",
            format: ImageFormat::Jpeg,
        })
}

fn screen_cache_format_from_image_format(format: ImageFormat) -> Option<ScreenCacheFormat> {
    match format {
        ImageFormat::Jpeg => Some(ScreenCacheFormat {
            extension: "jpg",
            format,
        }),
        ImageFormat::Png => Some(ScreenCacheFormat {
            extension: "png",
            format,
        }),
        ImageFormat::WebP => Some(ScreenCacheFormat {
            extension: "webp",
            format,
        }),
        _ => None,
    }
}

fn screen_cache_format_from_extension(path: &Path) -> Option<ScreenCacheFormat> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => Some(ScreenCacheFormat {
            extension: "jpg",
            format: ImageFormat::Jpeg,
        }),
        "png" => Some(ScreenCacheFormat {
            extension: "png",
            format: ImageFormat::Png,
        }),
        "webp" => Some(ScreenCacheFormat {
            extension: "webp",
            format: ImageFormat::WebP,
        }),
        _ => None,
    }
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
fn set_platform_wallpaper(path: &Path, layout: WallpaperLayoutPreference) -> Result<(), String> {
    let script = format!(
        "{}\n{}",
        macos_wallpaper_script(path),
        macos_layout_script(layout)
    );
    command_status(
        Command::new("osascript").arg("-e").arg(script).status(),
        "osascript",
    )
}

#[cfg(target_os = "linux")]
fn set_platform_wallpaper(path: &Path, layout: WallpaperLayoutPreference) -> Result<(), String> {
    let mut failures = Vec::new();

    for command in linux_wallpaper_commands(path, layout) {
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

#[cfg(target_os = "linux")]
fn set_platform_lock_screen_wallpaper(
    path: &Path,
    layout: WallpaperLayoutPreference,
) -> Result<(), String> {
    let mut failures = Vec::new();
    let mut attempted = false;

    for command in linux_lock_screen_commands(path, layout) {
        if !command_exists(&command.program) {
            continue;
        }

        attempted = true;
        match Command::new(&command.program).args(&command.args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => failures.push(format!("{} exited with {status}", command.program)),
            Err(error) => failures.push(format!("{} failed: {error}", command.program)),
        }
    }

    if !attempted {
        Err("No supported Linux lock-screen wallpaper tool was found.".into())
    } else if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

#[cfg(target_os = "windows")]
fn set_platform_lock_screen_wallpaper(
    _path: &Path,
    _layout: WallpaperLayoutPreference,
) -> Result<(), String> {
    Err(
        "Windows does not allow a reliable unsigned desktop app lock-screen wallpaper update."
            .into(),
    )
}

#[cfg(target_os = "macos")]
fn set_platform_lock_screen_wallpaper(
    _path: &Path,
    _layout: WallpaperLayoutPreference,
) -> Result<(), String> {
    Err("macOS does not expose a separate lock-screen wallpaper API for this app.".into())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn set_platform_lock_screen_wallpaper(
    _path: &Path,
    _layout: WallpaperLayoutPreference,
) -> Result<(), String> {
    Err("Lock-screen wallpaper changes are not supported on this operating system.".into())
}

pub fn linux_wallpaper_commands(
    path: &Path,
    layout: WallpaperLayoutPreference,
) -> Vec<WallpaperCommand> {
    let path_text = path.to_string_lossy().to_string();
    let uri = file_uri(path);
    let feh_arg = match layout {
        WallpaperLayoutPreference::Fit => "--bg-scale",
        WallpaperLayoutPreference::Stretch => "--bg-scale",
        WallpaperLayoutPreference::Tile => "--bg-tile",
        WallpaperLayoutPreference::Center => "--bg-center",
        WallpaperLayoutPreference::Fill | WallpaperLayoutPreference::Span => "--bg-fill",
    };
    let xwallpaper_arg = match layout {
        WallpaperLayoutPreference::Fit => "--maximize",
        WallpaperLayoutPreference::Stretch => "--stretch",
        WallpaperLayoutPreference::Tile => "--tile",
        WallpaperLayoutPreference::Center => "--center",
        WallpaperLayoutPreference::Fill | WallpaperLayoutPreference::Span => "--zoom",
    };
    vec![
        WallpaperCommand {
            program: "gsettings".into(),
            args: vec![
                "set".into(),
                "org.gnome.desktop.background".into(),
                "picture-options".into(),
                linux_picture_option(layout).into(),
            ],
        },
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
            args: vec![
                "img".into(),
                path_text.clone(),
                "--resize".into(),
                swww_resize_option(layout).into(),
            ],
        },
        WallpaperCommand {
            program: "feh".into(),
            args: vec![feh_arg.into(), path_text.clone()],
        },
        WallpaperCommand {
            program: "xwallpaper".into(),
            args: vec![xwallpaper_arg.into(), path_text],
        },
    ]
}

pub fn linux_lock_screen_commands(
    path: &Path,
    layout: WallpaperLayoutPreference,
) -> Vec<WallpaperCommand> {
    let uri = file_uri(path);
    vec![
        WallpaperCommand {
            program: "gsettings".into(),
            args: vec![
                "set".into(),
                "org.gnome.desktop.screensaver".into(),
                "picture-options".into(),
                linux_picture_option(layout).into(),
            ],
        },
        WallpaperCommand {
            program: "gsettings".into(),
            args: vec![
                "set".into(),
                "org.gnome.desktop.screensaver".into(),
                "picture-uri".into(),
                uri,
            ],
        },
    ]
}

pub fn linux_picture_option(layout: WallpaperLayoutPreference) -> &'static str {
    match layout {
        WallpaperLayoutPreference::Fill => "zoom",
        WallpaperLayoutPreference::Fit => "scaled",
        WallpaperLayoutPreference::Stretch => "stretched",
        WallpaperLayoutPreference::Tile => "wallpaper",
        WallpaperLayoutPreference::Center => "centered",
        WallpaperLayoutPreference::Span => "spanned",
    }
}

fn swww_resize_option(layout: WallpaperLayoutPreference) -> &'static str {
    match layout {
        WallpaperLayoutPreference::Fill | WallpaperLayoutPreference::Span => "crop",
        WallpaperLayoutPreference::Fit | WallpaperLayoutPreference::Center => "fit",
        WallpaperLayoutPreference::Stretch => "stretch",
        WallpaperLayoutPreference::Tile => "no",
    }
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
    let has_matching_aspect_ratio =
        image_width as u64 * screen_height as u64 == screen_width as u64 * image_height as u64;
    if image_width == 0
        || image_height == 0
        || screen_width == 0
        || screen_height == 0
        || !has_matching_aspect_ratio
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
    platform_screen_size()
}

#[cfg(target_os = "macos")]
fn platform_screen_size() -> Option<(u32, u32)> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"Finder\" to get bounds of window of desktop")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_macos_display_bounds(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "linux")]
fn platform_screen_size() -> Option<(u32, u32)> {
    Command::new("xrandr")
        .arg("--current")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                parse_xrandr_current(&String::from_utf8_lossy(&output.stdout))
            } else {
                None
            }
        })
        .or_else(|| {
            Command::new("xdpyinfo").output().ok().and_then(|output| {
                if output.status.success() {
                    parse_xdpyinfo_dimensions(&String::from_utf8_lossy(&output.stdout))
                } else {
                    None
                }
            })
        })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn platform_screen_size() -> Option<(u32, u32)> {
    None
}

pub fn screen_size_or_resolution(
    detected: Option<(u32, u32)>,
    resolution: ResolutionPreference,
) -> Option<(u32, u32)> {
    detected.or(match resolution {
        ResolutionPreference::Auto => None,
        ResolutionPreference::FullHd | ResolutionPreference::FourK => {
            Some(resolution.minimum_dimensions())
        }
    })
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

fn wallpaper_paths_match(left: &Path, right: &Path) -> bool {
    normalize_wallpaper_path(left) == normalize_wallpaper_path(right)
}

fn normalize_wallpaper_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if cfg!(target_os = "windows") {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

#[cfg(target_os = "windows")]
fn set_windows_wallpaper_style(layout: WallpaperLayoutPreference) -> Result<(), String> {
    for (name, value) in windows_layout_registry_values(layout) {
        let mut command = silent_platform_command("reg");
        let status = command
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

#[cfg(target_os = "windows")]
const WINDOWS_CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
fn silent_platform_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new(program);
    command.creation_flags(WINDOWS_CREATE_NO_WINDOW);
    command
}

fn file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", encode_uri_path(&normalized))
}

pub fn parse_macos_display_bounds(value: &str) -> Option<(u32, u32)> {
    let parts = value
        .split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }

    let width = (parts[2] - parts[0]).unsigned_abs();
    let height = (parts[3] - parts[1]).unsigned_abs();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width, height))
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_xrandr_current(value: &str) -> Option<(u32, u32)> {
    value.lines().find_map(|line| {
        if !line.contains('*') {
            return None;
        }
        line.split_whitespace().find_map(parse_dimensions)
    })
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_xdpyinfo_dimensions(value: &str) -> Option<(u32, u32)> {
    value.lines().find_map(|line| {
        line.trim()
            .strip_prefix("dimensions:")
            .and_then(|line| line.split_whitespace().next())
            .and_then(parse_dimensions)
    })
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_dimensions(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    if width == 0 || height == 0 {
        None
    } else {
        Some((width, height))
    }
}

pub fn parse_current_wallpaper_output(value: &str) -> Option<PathBuf> {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
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

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_current_wallpaper_script() -> &'static str {
    "tell application \"System Events\" to get picture of desktop 1"
}

#[cfg(target_os = "macos")]
fn macos_wallpaper_script(path: &Path) -> String {
    format!(
        "tell application \"System Events\" to set picture of every desktop to \"{}\"",
        escape_osascript_path(path)
    )
}

#[cfg(target_os = "macos")]
fn macos_layout_script(layout: WallpaperLayoutPreference) -> &'static str {
    match layout {
        WallpaperLayoutPreference::Tile => {
            "tell application \"System Events\" to set picture rotation of every desktop to 1"
        }
        _ => "tell application \"System Events\" to set picture rotation of every desktop to 0",
    }
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
    fn linux_commands_include_gnome_layout_and_common_wallpaper_tools() {
        let commands = linux_wallpaper_commands(
            Path::new("/home/me/Pictures/wall one.jpg"),
            WallpaperLayoutPreference::Fit,
        );

        assert!(commands.iter().any(|command| command.program == "gsettings"
            && command.args.contains(&"picture-uri".to_string())
            && command
                .args
                .contains(&"file:///home/me/Pictures/wall%20one.jpg".to_string())));
        assert!(commands.iter().any(|command| command.program == "gsettings"
            && command.args.contains(&"picture-options".to_string())
            && command.args.contains(&"scaled".to_string())));
        assert!(commands.iter().any(|command| command.program == "swww"));
        assert!(commands.iter().any(|command| command.program == "feh"));
        assert!(commands
            .iter()
            .any(|command| command.program == "xwallpaper"));
    }

    #[test]
    fn linux_layout_values_match_gnome_picture_options() {
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Fill),
            "zoom"
        );
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Fit),
            "scaled"
        );
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Stretch),
            "stretched"
        );
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Tile),
            "wallpaper"
        );
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Center),
            "centered"
        );
        assert_eq!(
            linux_picture_option(WallpaperLayoutPreference::Span),
            "spanned"
        );
    }

    #[test]
    fn linux_lock_screen_commands_target_gnome_screensaver() {
        let commands = linux_lock_screen_commands(
            Path::new("/home/me/Pictures/lock wall.jpg"),
            WallpaperLayoutPreference::Center,
        );

        assert!(commands.iter().any(|command| command.program == "gsettings"
            && command
                .args
                .contains(&"org.gnome.desktop.screensaver".to_string())
            && command.args.contains(&"picture-uri".to_string())
            && command
                .args
                .contains(&"file:///home/me/Pictures/lock%20wall.jpg".to_string())));
        assert!(commands.iter().any(|command| command.program == "gsettings"
            && command.args.contains(&"picture-options".to_string())
            && command.args.contains(&"centered".to_string())));
    }

    #[test]
    fn screen_size_falls_back_to_resolution_preference() {
        assert_eq!(
            screen_size_or_resolution(None, crate::settings::ResolutionPreference::FourK),
            Some((3840, 2160))
        );
        assert_eq!(
            screen_size_or_resolution(
                Some((2560, 1440)),
                crate::settings::ResolutionPreference::FullHd,
            ),
            Some((2560, 1440))
        );
    }

    #[test]
    fn parses_macos_display_bounds_from_osascript() {
        assert_eq!(
            parse_macos_display_bounds("0, 0, 3024, 1964"),
            Some((3024, 1964))
        );
    }

    #[test]
    fn parses_current_wallpaper_output() {
        assert_eq!(
            parse_current_wallpaper_output(" /Users/me/Pictures/wall.jpg\n"),
            Some(PathBuf::from("/Users/me/Pictures/wall.jpg"))
        );
        assert_eq!(
            parse_current_wallpaper_output("\"/Users/me/Pictures/wall one.jpg\"\n"),
            Some(PathBuf::from("/Users/me/Pictures/wall one.jpg"))
        );
        assert_eq!(parse_current_wallpaper_output(""), None);
    }

    #[test]
    fn macos_current_wallpaper_script_reads_plain_picture_path() {
        let script = macos_current_wallpaper_script();

        assert!(script.contains("get picture of desktop 1"));
        assert!(!script.contains("as alias"));
        assert!(!script.contains("POSIX path"));
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

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_registry_helpers_suppress_console_windows() {
        assert_eq!(WINDOWS_CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[test]
    fn same_aspect_ratio_oversized_wallpapers_are_sized_inside_screen_without_upscaling() {
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((3840, 2160), (1920, 1080)),
            Some((1920, 1080))
        );
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((4096, 2304), (1920, 1080)),
            Some((1920, 1080))
        );
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((1280, 720), (1920, 1080)),
            None
        );
    }

    #[test]
    fn different_aspect_ratio_wallpapers_keep_original_size_to_avoid_fill_blur() {
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((4000, 3000), (1920, 1080)),
            None
        );
        assert_eq!(
            fit_wallpaper_to_screen_dimensions((3000, 4000), (1080, 1920)),
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

    #[test]
    fn screen_resize_preserves_supported_source_format() {
        let dir = temp_dir("screen-resize-png");
        let output_dir = dir.join("screen");
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        let source = dir.join("source.png");
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            400,
            200,
            image::Rgba([10, 20, 30, 160]),
        ));
        image.save(&source).expect("source image should save");

        let resized =
            resize_wallpaper_for_screen(&source, &output_dir, (200, 100)).expect("should resize");

        assert_eq!(
            resized.extension().and_then(|value| value.to_str()),
            Some("png")
        );
        assert_eq!(
            image::ImageReader::open(&resized)
                .expect("resized image should open")
                .with_guessed_format()
                .expect("format should be guessed")
                .format(),
            Some(ImageFormat::Png)
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn wallpaper_lock_restores_when_current_wallpaper_differs() {
        let locked = WallpaperLock {
            path: PathBuf::from("C:\\Wallpapers\\app-wallpaper.jpg"),
            layout: WallpaperLayoutPreference::Fit,
        };
        let mut applied = Vec::new();

        let restored = restore_locked_wallpaper_if_needed(
            &locked,
            Some(PathBuf::from(
                "C:\\Windows\\Web\\Wallpaper\\Windows\\img0.jpg",
            )),
            |path, layout| {
                applied.push((path.to_path_buf(), layout));
                Ok(())
            },
        )
        .expect("wallpaper lock should restore");

        assert!(restored);
        assert_eq!(
            applied,
            vec![(locked.path.clone(), WallpaperLayoutPreference::Fit)]
        );
    }

    #[test]
    fn wallpaper_lock_does_not_restore_when_current_wallpaper_matches() {
        let locked = WallpaperLock {
            path: PathBuf::from("C:\\Wallpapers\\app-wallpaper.jpg"),
            layout: WallpaperLayoutPreference::Fit,
        };
        let mut applied = Vec::new();

        let restored = restore_locked_wallpaper_if_needed(
            &locked,
            Some(PathBuf::from("C:\\Wallpapers\\app-wallpaper.jpg")),
            |path, layout| {
                applied.push((path.to_path_buf(), layout));
                Ok(())
            },
        )
        .expect("wallpaper lock should not restore");

        assert!(!restored);
        assert!(applied.is_empty());
    }

    #[test]
    fn wallpaper_lock_can_start_from_current_desktop_wallpaper() {
        let current = PathBuf::from("C:\\Wallpapers\\current.jpg");

        assert_eq!(
            wallpaper_lock_from_current_desktop(
                Some(current.clone()),
                WallpaperLayoutPreference::Fill
            ),
            Some(WallpaperLock {
                path: current,
                layout: WallpaperLayoutPreference::Fill
            })
        );
        assert_eq!(
            wallpaper_lock_from_current_desktop(None, WallpaperLayoutPreference::Fill),
            None
        );
    }

    #[test]
    fn shutdown_restore_returns_to_startup_wallpaper_after_app_changes_wallpaper() {
        let startup = WallpaperLock {
            path: PathBuf::from("C:\\Wallpapers\\before-app.jpg"),
            layout: WallpaperLayoutPreference::Fit,
        };
        let active = WallpaperLock {
            path: PathBuf::from("C:\\Wallpapers\\app-wallpaper.jpg"),
            layout: WallpaperLayoutPreference::Fill,
        };
        let mut applied = Vec::new();

        let restored = restore_startup_wallpaper_if_app_changed(
            Some(&startup),
            Some(&active),
            Some(active.path.clone()),
            |path, layout| {
                applied.push((path.to_path_buf(), layout));
                Ok(())
            },
        )
        .expect("startup wallpaper should restore");

        assert!(restored);
        assert_eq!(
            applied,
            vec![(startup.path.clone(), WallpaperLayoutPreference::Fit)]
        );
    }

    #[test]
    fn shutdown_restore_skips_when_app_never_changed_wallpaper() {
        let startup = WallpaperLock {
            path: PathBuf::from("C:\\Wallpapers\\before-app.jpg"),
            layout: WallpaperLayoutPreference::Fit,
        };
        let mut applied = Vec::new();

        let restored = restore_startup_wallpaper_if_app_changed(
            Some(&startup),
            Some(&startup),
            Some(startup.path.clone()),
            |path, layout| {
                applied.push((path.to_path_buf(), layout));
                Ok(())
            },
        )
        .expect("startup wallpaper should not need restore");

        assert!(!restored);
        assert!(applied.is_empty());
    }
}
