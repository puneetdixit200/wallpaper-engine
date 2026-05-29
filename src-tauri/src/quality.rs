use crate::models::{Wallpaper, WallpaperQualityReport};
use crate::settings::{AppSettings, QualityGuardMode};

pub fn assess_wallpaper_quality(
    wallpaper: &Wallpaper,
    settings: &AppSettings,
) -> WallpaperQualityReport {
    let mut warnings = Vec::new();

    if wallpaper.width == 0 || wallpaper.height == 0 {
        warnings.push("Source resolution is unknown.".to_string());
    } else {
        if wallpaper.width < settings.quality_min_width {
            warnings.push(format!("Width is below {} px.", settings.quality_min_width));
        }
        if wallpaper.height < settings.quality_min_height {
            warnings.push(format!(
                "Height is below {} px.",
                settings.quality_min_height
            ));
        }
        if wallpaper.height > wallpaper.width && !settings.allow_portrait_wallpapers {
            warnings.push("Portrait wallpapers are disabled.".to_string());
        }
        if wallpaper.width.saturating_mul(100) / wallpaper.height.max(1) < 120 {
            warnings.push("Aspect ratio is narrow for most desktop screens.".to_string());
        }
    }

    WallpaperQualityReport {
        ok: warnings.is_empty(),
        warnings,
    }
}

pub fn should_skip_wallpaper(wallpaper: &Wallpaper, settings: &AppSettings) -> bool {
    settings.quality_guard_mode == QualityGuardMode::Skip
        && !assess_wallpaper_quality(wallpaper, settings).ok
}

pub fn quality_error_message(report: &WallpaperQualityReport) -> String {
    if report.warnings.is_empty() {
        "Wallpaper did not pass the quality guard.".into()
    } else {
        format!(
            "Wallpaper did not pass the quality guard: {}",
            report.warnings.join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Wallpaper;
    use crate::settings::{AppSettings, QualityGuardMode};

    fn wallpaper(width: u32, height: u32) -> Wallpaper {
        Wallpaper {
            id: "quality".into(),
            source: "test".into(),
            thumb_url: String::new(),
            full_url: String::new(),
            photographer: String::new(),
            width,
            height,
            query_used: None,
            mood: None,
            local_path: None,
            is_favorite: false,
        }
    }

    #[test]
    fn flags_small_or_portrait_wallpapers() {
        let settings = AppSettings::default();
        let report = assess_wallpaper_quality(&wallpaper(900, 1600), &settings);

        assert!(!report.ok);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("Width")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("Portrait")));
    }

    #[test]
    fn skip_mode_rejects_low_quality_wallpapers() {
        let settings = AppSettings {
            quality_guard_mode: QualityGuardMode::Skip,
            ..AppSettings::default()
        };

        assert!(should_skip_wallpaper(&wallpaper(1280, 720), &settings));
        assert!(!should_skip_wallpaper(&wallpaper(3840, 2160), &settings));
    }
}
