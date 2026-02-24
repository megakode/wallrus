

# Wallrus

<p align="center">
  <img src="data/icons/io.github.megakode.Wallrus.svg" width="128" height="128" alt="Wallrus icon">
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
- **Custom palettes** — tweak individual colors with the color pickers, then save
  your palette for later. Saved palettes appear in a "Custom" category and can
  be deleted at any time.
- **Blend control** — go from hard flag-like stripes to fully smooth gradients
- **Effects** — Distortion, lighting, and noise
- **Export** — PNG or JPEG at 1080p, 1440p, or 4K via a native save dialog
  (defaults to your Pictures folder; resolution auto-detected from your display)
- **Set as wallpaper** — uses the XDG Desktop Portal to set your GNOME wallpaper
- **Keyboard shortcuts** — Ctrl+E (export), Ctrl+Shift+W (set as wallpaper)

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

### Nix

A [Nix flake](https://nix.dev/concepts/flakes) is provided. You can run
Wallrus directly without installing:

```
nix run github:megakode/wallrus
```

To add it to a NixOS configuration:

```nix
# flake.nix
{
  inputs.wallrus.url = "github:megakode/wallrus";
  # ...
}

# configuration.nix
environment.systemPackages = [
  inputs.wallrus.packages.${pkgs.system}.default
];
```

A development shell with all native dependencies, `rust-analyzer`, `clippy`,
and `rustfmt` is also available:

```
nix develop
```

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.
