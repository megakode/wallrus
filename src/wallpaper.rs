use std::fs::File;
use std::os::fd::AsFd;
use std::path::Path;

use ashpd::desktop::wallpaper::{SetOn, WallpaperRequest};
use ashpd::WindowIdentifier;

/// Set the desktop wallpaper using the XDG Desktop Portal.
///
/// The image at `path` is opened and passed as a file descriptor to the portal.
/// The portal may show a preview dialog to the user before applying.
///
/// A valid `WindowIdentifier` is obtained from the given GTK4 window so that
/// the portal can associate the request with the calling application. This is
/// required for the wallpaper portal to work correctly inside a Flatpak sandbox.
pub async fn set_wallpaper(
    path: &Path,
    window: &impl gtk4::prelude::IsA<gtk4::Native>,
) -> Result<(), String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open wallpaper file: {}", e))?;

    let identifier = WindowIdentifier::from_native(window).await;

    WallpaperRequest::default()
        .identifier(identifier)
        .set_on(SetOn::Both)
        .show_preview(true)
        .build_file(&file.as_fd())
        .await
        .map_err(|e| format!("wallpaper portal error: {}", e))?
        .response()
        .map_err(|e| format!("wallpaper portal response error: {}", e))
}
