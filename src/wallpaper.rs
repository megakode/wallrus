use std::fs::File;
use std::os::fd::AsFd;
use std::path::Path;

use ashpd::desktop::wallpaper::{SetOn, WallpaperRequest};

/// Set the desktop wallpaper using the XDG Desktop Portal.
///
/// The image at `path` is opened and passed as a file descriptor to the portal.
/// The portal may show a preview dialog to the user before applying.
pub async fn set_wallpaper(path: &Path) -> Result<(), String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open wallpaper file: {}", e))?;

    let request = WallpaperRequest::default()
        .set_on(SetOn::Both)
        .show_preview(true)
        .build_file(&file.as_fd())
        .await
        .map_err(|e| format!("Wallpaper portal error: {}", e))?;

    request
        .response()
        .map_err(|e| format!("Wallpaper portal response error: {}", e))
}
