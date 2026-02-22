

# Wallrus

<p align="center">
  <img src="data/icons/com.megakode.Wallrus.svg" width="128" height="128" alt="Wallrus icon">
</p>
A user-friendly GNOME (GTK4) application for generating colorful abstract wallpapers based on different patterns and effects.

For those who prefer minimalist, colorful wallpapers and like changing colors once in a while to keep things fresh.

<img width="1346" height="944" alt="Screenshot From 2026-02-22 15-04-59" src="https://github.com/user-attachments/assets/24508c47-0103-4817-a92f-bfd632fc9b67" />


## Example wallpapers

![wallrus_bars_1771764551](https://github.com/user-attachments/assets/880a55a9-7b80-4d2c-86c9-3dd51da19a89)
![wallrus_bars_1771764536](https://github.com/user-attachments/assets/0034302e-1204-4ff7-8cf9-97c80ae61de6)
![wallrus_waves_1771764682](https://github.com/user-attachments/assets/898a8c7d-07df-46fc-b939-60f2d840ff38)
![wallrus_terrain_1771764802](https://github.com/user-attachments/assets/95c61466-47df-45a0-92fe-bd17d4bb6c88)
![wallrus_terrain_1771764772](https://github.com/user-attachments/assets/bf52c0e1-2557-4af6-aa68-09e5253862e5)
![wallrus_circle_1771764645](https://github.com/user-attachments/assets/f5ef79bd-56b1-4561-9c0c-65b11512611f)



## Features

- **5 shader presets** — Bars, Circle, Plasma, Waves, and Terrain, each with
  dedicated parameters (angle, scale, time scrub, center position)
- **Hundreds of bundled palette images** across several categories (cold, dark, fall,
  gradient, light, pastel, retro, sunset, warm, winter, etc.)
- **Blend control** — go from hard flag-like stripes to fully smooth gradients
- **Effects** - Distortion, lightning and noise
- **Export** — PNG or JPEG at 1080p, 1440p, or 4K (default auto-detected from
  your display)
- **Set as wallpaper** — writes to `~/.local/share/backgrounds/` and applies
  via `gsettings`.
- **Ligh / Dark mode** - Set either light or dark wallpaper when exporting, to easily tweak and create a matching pair.
- **Custom palettes** - drop 1×4 px palette images into
  `~/.local/share/wallrus/palettes/<category>/` and they appear automatically
- **Keyboard shortcuts** — Ctrl+E (export PNG), Ctrl+Shift+E (export JPEG),
  Ctrl+Shift+W (set as wallpaper)

## Requirements

- GTK 4 (≥ 4.10)
- libadwaita (≥ 1.4)
- OpenGL 3.3+ capable GPU
- Rust 1.70+

System packages (Fedora):

```
sudo dnf install gtk4-devel libadwaita-devel
```

System packages (Ubuntu/Debian):

```
sudo apt install libgtk-4-dev libadwaita-1-dev
```

## Building

```
cargo build --release
```

The binary is at `target/release/wallrus`.

## Installing

The included install script builds a release binary and copies everything to
`~/.local` (binary, desktop file, icon, metainfo, and bundled palettes):

```
./install.sh
```

To install to a different prefix:

```
PREFIX=/usr/local ./install.sh
```

You may need to log out and back in for the application icon to appear in your
launcher.

## Custom palettes

Palette images are 1x4 px PNGs — one pixel per color, top to bottom (4 colors
total). Wallrus scales them to 80x80 thumbnails in the UI and reads each pixel
directly to extract colors.

Place them in subdirectories under `~/.local/share/wallrus/palettes/`:

```
~/.local/share/wallrus/palettes/
├── mytheme/
│   ├── ocean.png
│   └── forest.png
└── another-category/
    └── fire.png
```

Subdirectory names become selectable categories in the UI (capitalized
automatically). Restart Wallrus to pick up new palettes.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.
