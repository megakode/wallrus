/// Palette image handling — extract colors from 1x4 palette images
/// and list available palette images organized by category (subfolder).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A category name mapped to its palette image paths (sorted by filename).
pub type PaletteCategories = BTreeMap<String, Vec<PathBuf>>;

/// Extract 4 colors from a palette image.
///
/// The image is expected to be 1x4px (one pixel per color, top to bottom).
/// For backward compatibility, larger images are also supported — the image
/// is divided into 4 equal horizontal bands and the center pixel of each
/// band is sampled.
pub fn extract_colors_from_image(path: &Path) -> Result<[[f32; 3]; 4], String> {
    let img = image::open(path).map_err(|e| format!("Failed to load image: {}", e))?;
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    if width == 0 || height == 0 {
        return Err("Image has zero dimensions".to_string());
    }

    let cx = width / 2;
    let band_height = height / 4;

    let mut colors = [[0.0f32; 3]; 4];
    for i in 0..4 {
        let cy = band_height * i as u32 + band_height / 2;
        let cy = cy.min(height - 1);
        let pixel = rgb.get_pixel(cx, cy);
        colors[i] = [
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        ];
    }

    Ok(colors)
}

/// List all palette images organized by category.
///
/// Categories are subfolders inside the palette root directories.
/// Images directly in the root (not in a subfolder) go into an "Uncategorized" category.
/// Both bundled and user directories are scanned and merged.
/// Categories are sorted alphabetically; images within each are sorted by filename.
pub fn list_palette_categories() -> PaletteCategories {
    let mut categories: PaletteCategories = BTreeMap::new();

    if let Some(dir) = bundled_palettes_dir() {
        collect_categorized_images(&dir, &mut categories);
    }

    if let Some(dir) = user_palettes_dir() {
        collect_categorized_images(&dir, &mut categories);
    }

    // Sort images within each category by filename
    for images in categories.values_mut() {
        images.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });
    }

    categories
}

/// Get the bundled palettes directory.
///
/// Looks for palettes relative to the executable, then falls back to
/// common installation paths. During development this is `data/palettes/`
/// relative to the project root.
pub fn bundled_palettes_dir() -> Option<PathBuf> {
    // During development: look relative to the executable
    if let Ok(exe) = std::env::current_exe() {
        // target/debug/wallrus -> project_root/data/palettes
        if let Some(project_root) = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let dev_path = project_root.join("data").join("palettes");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
        }
    }

    // Installed (prefix-relative): <prefix>/bin/wallrus -> <prefix>/share/wallrus/palettes
    if let Ok(exe) = std::env::current_exe() {
        if let Some(prefix) = exe.parent().and_then(|p| p.parent()) {
            let prefix_path = prefix.join("share").join("wallrus").join("palettes");
            if prefix_path.is_dir() {
                return Some(prefix_path);
            }
        }
    }

    // Installed: /usr/share/wallrus/palettes
    let system_path = PathBuf::from("/usr/share/wallrus/palettes");
    if system_path.is_dir() {
        return Some(system_path);
    }

    // Flatpak or local: /app/share/wallrus/palettes
    let flatpak_path = PathBuf::from("/app/share/wallrus/palettes");
    if flatpak_path.is_dir() {
        return Some(flatpak_path);
    }

    None
}

/// Get the user palettes directory (~/.local/share/wallrus/palettes/).
/// Returns the path only if the directory already exists.
pub fn user_palettes_dir() -> Option<PathBuf> {
    let data_dir = dirs::data_dir()?;
    let palette_dir = data_dir.join("wallrus").join("palettes");
    if palette_dir.is_dir() {
        Some(palette_dir)
    } else {
        None
    }
}

/// Scan a palette root directory for categorized images.
///
/// - Subfolders become categories (folder name with first letter capitalized).
/// - Image files directly in the root go into "Uncategorized".
fn collect_categorized_images(root: &Path, categories: &mut PaletteCategories) {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Subfolder = category
            let category_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(capitalize_first)
                .unwrap_or_else(|| "Uncategorized".to_string());

            let sub_entries = match std::fs::read_dir(&path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for sub_entry in sub_entries.flatten() {
                let sub_path = sub_entry.path();
                if sub_path.is_file() && is_image_file(&sub_path) {
                    categories
                        .entry(category_name.clone())
                        .or_default()
                        .push(sub_path);
                }
            }
        } else if path.is_file() && is_image_file(&path) {
            // Files directly in root go to "Uncategorized"
            categories
                .entry("Uncategorized".to_string())
                .or_default()
                .push(path);
        }
    }
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
        .unwrap_or(false)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
