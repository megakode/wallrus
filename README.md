# Wallrus

A user-friendly GNOME (GTK4) application for generating colorful abstract wallpapers based on different patterns and effects.

For those who prefer minimalist, colorful wallpapers and like changing colors once in a while to keep things fresh.

<img width="1164" height="753" alt="Screenshot From 2026-02-22 11-51-12" src="https://github.com/user-attachments/assets/09dd81eb-cb49-47dc-89d3-fca411416c53" />



## Example wallpapers
![wallrus_terrain_1771756626](https://github.com/user-attachments/assets/2e8b0ea2-9be6-43ee-8d9c-2e3e74b126e9)
![wallrus_terrain_1771756619](https://github.com/user-attachments/assets/63703046-5cd5-4c22-b9e1-84f866ff113b)
![wallrus_plasma_1771756557](https://github.com/user-attachments/assets/f1bbb1e9-d360-4270-9c8c-683e3b7b21f6)
![wallrus_bars_1771756517](https://github.com/user-attachments/assets/0583e2d6-e3d9-4a4f-9286-463cb33a6746)
![wallrus_bars_1771756489](https://github.com/user-attachments/assets/f12ec4db-34b7-400f-9e0e-4159d2e00d77)

## Features

- **5 shader presets** — Bars, Circle, Plasma, Waves, and Terrain, each with
  dedicated parameters (angle, scale, time scrub, center position)
- **Hundreds of bundled palette images** across several categories (cold, dark, fall,
  gradient, light, pastel, retro, sunset, warm, winter, etc.)
- **Blend control** — go from hard flag-like stripes to fully smooth gradients
- **Effects** — swirl distortion, film grain noise, and ordered Bayer dithering
  for a retro look
- **Export** — PNG or JPEG at 1080p, 1440p, or 4K (default auto-detected from
  your display)
- **Set as wallpaper** — writes to `~/.local/share/backgrounds/` and applies
  via `gsettings` for both light and dark mode
- **Custom palettes** — drop 400×400 px palette images into
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

Palette images are 400×400 px PNGs with four horizontal color bands (100 px
each, top to bottom). Wallrus samples the center pixel of each band to extract
the four colors.

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
