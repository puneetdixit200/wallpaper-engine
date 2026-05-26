use crate::settings::WallpaperLayoutPreference;
use std::path::Path;

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
    use std::path::Path;

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
}
