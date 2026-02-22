use std::path::Path;
use std::process::Command;

/// Which wallpaper mode(s) to set.
#[derive(Debug, Clone, Copy)]
pub enum WallpaperMode {
    Both,
    LightOnly,
    DarkOnly,
}

/// Set the GNOME desktop wallpaper using gsettings.
/// Takes separate paths for light and dark mode so GNOME treats them as
/// distinct files and correctly updates thumbnails for both appearances.
pub fn set_gnome_wallpaper(
    light_path: &Path,
    dark_path: &Path,
    mode: WallpaperMode,
) -> Result<(), String> {
    match mode {
        WallpaperMode::Both => {
            let light_uri = path_to_uri(light_path)?;
            let dark_uri = path_to_uri(dark_path)?;
            run_gsettings("picture-uri", &light_uri)?;
            run_gsettings("picture-uri-dark", &dark_uri)?;
        }
        WallpaperMode::LightOnly => {
            let uri = path_to_uri(light_path)?;
            run_gsettings("picture-uri", &uri)?;
        }
        WallpaperMode::DarkOnly => {
            let uri = path_to_uri(dark_path)?;
            run_gsettings("picture-uri-dark", &uri)?;
        }
    }
    Ok(())
}

fn path_to_uri(path: &Path) -> Result<String, String> {
    let abs = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    Ok(format!("file://{}", abs.display()))
}

fn run_gsettings(key: &str, value: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", key, value])
        .output()
        .map_err(|e| format!("Failed to run gsettings: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gsettings failed for {}: {}", key, stderr));
    }

    Ok(())
}
