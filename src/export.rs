use image::{ImageBuffer, Rgba};

/// Export resolution presets
#[derive(Debug, Clone, Copy)]
pub enum ExportResolution {
    Display(u32, u32), // Current monitor resolution
    Hd,                // 1920x1080
    Qhd,               // 2560x1440
    Uhd4k,             // 3840x2160
    Phone,             // 1080x2400 (portrait, 9:20)
}

impl ExportResolution {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            ExportResolution::Display(w, h) => (w, h),
            ExportResolution::Hd => (1920, 1080),
            ExportResolution::Qhd => (2560, 1440),
            ExportResolution::Uhd4k => (3840, 2160),
            ExportResolution::Phone => (1080, 2400),
        }
    }

    /// Build from ComboRow index. Index 0 = Display (requires dimensions),
    /// 1 = HD, 2 = QHD, 3 = 4K, 4 = Phone.
    pub fn from_index(index: u32, display_dims: (u32, u32)) -> Self {
        match index {
            0 => ExportResolution::Display(display_dims.0, display_dims.1),
            1 => ExportResolution::Hd,
            2 => ExportResolution::Qhd,
            3 => ExportResolution::Uhd4k,
            4 => ExportResolution::Phone,
            _ => ExportResolution::Hd,
        }
    }
}

/// Export format
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Png,
    Jpeg,
}

impl ExportFormat {
    /// Infer format from a file extension string.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => ExportFormat::Jpeg,
            _ => ExportFormat::Png,
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
