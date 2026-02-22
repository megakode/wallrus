use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

/// Export resolution presets
#[derive(Debug, Clone, Copy)]
pub enum ExportResolution {
    Hd,    // 1920x1080
    Qhd,   // 2560x1440
    Uhd4k, // 3840x2160
}

impl ExportResolution {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            ExportResolution::Hd => (1920, 1080),
            ExportResolution::Qhd => (2560, 1440),
            ExportResolution::Uhd4k => (3840, 2160),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExportResolution::Hd => "1080p (1920x1080)",
            ExportResolution::Qhd => "1440p (2560x1440)",
            ExportResolution::Uhd4k => "4K (3840x2160)",
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            0 => ExportResolution::Hd,
            1 => ExportResolution::Qhd,
            2 => ExportResolution::Uhd4k,
            _ => ExportResolution::Hd,
        }
    }

    /// Returns the index of the resolution preset closest to the given
    /// display dimensions (by total pixel count).
    pub fn best_index_for_display(width: i32, height: i32) -> u32 {
        let display_pixels = (width as u64) * (height as u64);
        let resolutions = [
            ExportResolution::Hd,
            ExportResolution::Qhd,
            ExportResolution::Uhd4k,
        ];
        let mut best_idx = 0u32;
        let mut best_diff = u64::MAX;
        for (i, res) in resolutions.iter().enumerate() {
            let (w, h) = res.dimensions();
            let pixels = (w as u64) * (h as u64);
            let diff = display_pixels.abs_diff(pixels);
            if diff < best_diff {
                best_diff = diff;
                best_idx = i as u32;
            }
        }
        best_idx
    }
}

/// Export format
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Png,
    Jpeg,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
        }
    }
}

/// Save RGBA pixel data to an image file
pub fn save_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
    path: &std::path::Path,
    format: ExportFormat,
) -> Result<(), String> {
    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, pixels.to_vec())
        .ok_or("Failed to create image buffer from pixel data")?;

    match format {
        ExportFormat::Png => {
            img.save(path)
                .map_err(|e| format!("Failed to save PNG: {}", e))?;
        }
        ExportFormat::Jpeg => {
            // Convert RGBA to RGB for JPEG
            let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
            rgb_img
                .save(path)
                .map_err(|e| format!("Failed to save JPEG: {}", e))?;
        }
    }

    Ok(())
}

/// Get the default export directory, creating it if needed
pub fn default_export_dir() -> Result<PathBuf, String> {
    let pictures = dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Pictures")))
        .ok_or("Could not determine pictures directory")?;

    let export_dir = pictures.join("Wallrus");
    if !export_dir.exists() {
        std::fs::create_dir_all(&export_dir)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
    }

    Ok(export_dir)
}
