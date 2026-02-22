# Wallrus — Agent Context

## Goal

Build a GNOME application called **Wallrus** (app ID: `com.megakode.Wallrus`) that generates abstract wallpapers using GPU shaders. It has a GTK4/libadwaita GUI with a live shader preview, **palette-image-based color selection** (browsing 400x400px palette images organized in category subfolders, displayed as thumbnails in a grid), parameter controls, multiple shader presets, image export, and the ability to set the GNOME desktop wallpaper directly.

## Instructions

- **Language:** Rust
- **UI Framework:** GTK4 with libadwaita for modern GNOME styling
- **Shader rendering:** Use `GtkGLArea` with `glow` crate for OpenGL bindings, GLSL 330 core shaders
- **Shader presets:** Bars (with angle), Circle (scale/center), Plasma (scale/time), Waves (angle/scale/time), Terrain (scale/time) — each with appropriate configurable parameters.
- **All shaders use exactly 4 colors** from palette images (no `uColorCount` — always 4 colors)
- **All shaders have a Blend parameter** (`uBlend` uniform, range 0.0–1.0, default 0.5) that controls transition sharpness between color bands. At 0 = hard flag-like stripes with pixel-sharp edges. At 1 = fully smooth blending. Uses `smoothstep` with variable-width transition zones at boundaries 0.25, 0.5, 0.75. Blend slider has "hard" / "smooth" hint labels below it.
- **Effects section** — A separate `adw::PreferencesGroup` titled "Effects" in the **right column** (below Preview, above Export). Contains:
  - **Swirl** (`uSwirl` uniform, range -10.0 to +10.0, default 0.0) — vortex distortion applied to UV coordinates. Has "left" / "right" hint labels.
  - **Noise** (`uNoise` uniform, range -1.0 to +1.0, default 0.0) — film grain effect. Negative = darker grain, positive = lighter grain. Has "darker" / "lighter" hint labels.
  - **Dither** (`uDither` uniform, 0.0 or 1.0) — ordered Bayer 4x4 dithering, quantizes to 4 levels per channel for a retro pixel art look. Controlled by a `gtk4::Switch` toggle (on/off).
- **Hint labels pattern:** Small dim gray text below sliders using a `gtk4::Box` with two `gtk4::Label`s (css classes `dim-label` + `caption`), wrapped in a non-activatable/non-selectable `gtk4::ListBoxRow`, added to the PreferencesGroup after the slider row.
- **No shaders animate continuously.** Plasma and Waves both use `uSpeed` as a **static time scrub value**. The slider is labeled "Time" (range 0–20, default 0) for Plasma, Waves, and Terrain. Bars doesn't use time at all.
- **Palette system:** Users browse 400x400px palette images (4 horizontal color bands, 100px each). Colors extracted by sampling center pixel of each band at y=50,150,250,350. Displayed as 80x80px thumbnails in a `GtkFlowBox` with 200px fixed-height scrollable area. The FlowBox scrolled window is wrapped in a `gtk4::ListBoxRow` so it renders inside the PreferencesGroup's rounded rectangle together with the category dropdown.
- **Category system:** Palette images are organized in **subfolders** within the palette directories. Subfolders become categories shown in a dropdown above the FlowBox. Selecting a category repopulates the FlowBox. Files directly in the root go to "Uncategorized". Category names are capitalized.
- **Palette image locations:** Both bundled (`data/palettes/`) AND user directory (`~/.local/share/wallrus/palettes/`).
- **No manual color pickers** — colors come exclusively from selecting palette image thumbnails.
- **Export:** PNG and JPEG at 1080p, 1440p, and 4K resolutions, saved to `~/Pictures/Wallrus/`. Default resolution auto-detected from the current display.
- **Wallpaper integration:** Save to `~/.local/share/backgrounds/` and set via `gsettings` (both light and dark mode URIs)
- **Keyboard shortcuts:** Ctrl+E (Export PNG), Ctrl+Shift+E (Export JPEG), Ctrl+Shift+W (Set as Wallpaper)
- **Layout:** Two-column layout. Left column (scrollable, 320px min width): Palette group + Pattern controls group. Right column (expanding): Preview group + Effects group + Export group + buttons. Window default size 1100x700.
- **Both columns use `adw::PreferencesGroup`** for consistent styled section headers with rounded rectangles.
- **`PresetControls` struct** has fields: `has_angle`, `has_scale`, `has_speed`, `has_center`, `speed_label`, `speed_range`, `scale_range`. The UI updates label, range, visibility, and defaults when switching presets.
- **Per-preset scale ranges:** The `scale_range` field on `PresetControls` allows each shader to define its own scale slider range. Terrain uses 0.1–2.0, Circle uses 0.5–3.0, others use 0.1–5.0.
- **Pattern section** — titled "Pattern" in the UI, contains Type dropdown + parameter sliders.
- **App name:** Wallrus, **App ID:** `com.megakode.Wallrus`, **Author:** megakode

## Discoveries

- **Epoxy linking issue:** The original approach tried to use `epoxy_get_proc_address` from libepoxy, but this symbol doesn't exist in epoxy 1.5.10. Solution: Use `dlopen`/`dlsym` at runtime to load `eglGetProcAddress` from `libEGL.so.1` (Wayland) or `glXGetProcAddressARB` from `libGLX.so.0` (X11). This is the `gl_loader` module in `gl_renderer.rs`.
- **GTK4 crate feature flags:** `ColorDialogButton` and `ColorDialog` require `v4_10` feature on `gtk4` crate. `ToolbarView` requires `v1_4` feature on `libadwaita` crate.
- **Do NOT use `set_required_version(3, 3)` on GLArea** — it causes "Unable to create a GL context" errors on Wayland/NVIDIA. GTK4 defaults to a compatible version and the shaders work fine without it.
- **`gtk4::Picture::for_paintable()` takes a reference, not `Option`** — use `&texture` not `Some(&texture)`.
- **System environment:** GTK4 4.20.3, libadwaita 1.8.4, epoxy 1.5.10, Rust 1.86.0, Wayland session, NVIDIA 590.48.01 (OpenGL 4.6), x86_64 Linux.
- **Crate versions used:** gtk4 0.9 (v4_10), libadwaita 0.7 (v1_4), glow 0.14, image 0.25, dirs 5.0, libc 0.2.
- **Adwaita warning** `"Using GtkSettings:gtk-application-prefer-dark-theme with libadwaita is unsupported"` appears on stderr — this is from the system settings, not our code. Harmless, ignore it.
- **ToastOverlay** must be created at window construction time wrapping the ToolbarView, not dynamically on first toast.
- **Blend/sharpness control:** The original "steepness" approach using `pow(t, steepness)` did NOT work. The correct approach uses 4 equal color bands with boundaries at 0.25, 0.5, 0.75, and `smoothstep` with variable-width fade zones controlled by `uBlend`.
- **Swirl UV distortion:** `vec2 swirlUV(vec2 uv)` rotates UV around center by `uSwirl * (1.0 - distance)`. Applied at the start of every shader's `main()`.
- **Terrain shader smoothness:** Value noise was too chaotic. Gradient noise with quintic interpolation (`6t^5 - 15t^4 + 10t^3`) + single octave + double smoothstep post-processing produces smooth rounded contour hills. The `hash2()` function returns 2D gradient vectors for the gradient noise.
- **Gradient noise range:** `gnoise()` returns values that cluster around 0.3–0.7 (not full 0–1), so terrain height needs remapping: `clamp((height - 0.15) * 1.4, 0.0, 1.0)` followed by double smoothstep.
- **Noise grain direction:** `hash()` returns 0–1, multiplied by `uNoise` directly (not centered at 0). Positive uNoise adds brightness, negative subtracts. This gives directional grain control with the -1 to +1 slider.
- **Edit matching issues:** The shader files have multiple identical code blocks (shared paletteColor/swirlUV/hash functions and noise grain application). When editing, always include enough surrounding unique context to distinguish which shader's block is being modified.
- **Dither implementation:** 4x4 ordered Bayer matrix dithering, quantizing to 4 levels per channel. Applied after noise grain, before final `fragColor` output. Controlled by `uDither` uniform (0.0 = off, 1.0 = on). UI is a simple `gtk4::Switch` toggle in the Effects group.

## Relevant files / directories

- `/home/sbeam/code/wallpaper/Cargo.toml` — Project config (gtk4 0.9 w/ v4_10, libadwaita 0.7 w/ v1_4, glow 0.14, image 0.25, dirs 5.0, libc 0.2). 19 lines.
- `/home/sbeam/code/wallpaper/src/main.rs` — Entry point (has `mod palette`). 17 lines.
- `/home/sbeam/code/wallpaper/src/application.rs` — AdwApplication setup. 31 lines.
- `/home/sbeam/code/wallpaper/src/palette.rs` — Category-aware palette image extraction + directory listing. Scans bundled `data/palettes/` and user `~/.local/share/wallrus/palettes/`. 178 lines.
- `/home/sbeam/code/wallpaper/src/gl_renderer.rs` — GL context, RendererState (all uniform fields: color1-4, angle, scale, speed, blend, swirl, noise, center, dither), fullscreen quad, render-to-pixels. Contains `gl_loader` module for EGL/GLX dynamic loading. 370 lines.
- `/home/sbeam/code/wallpaper/src/shader_presets.rs` — 5 shader presets (Bars, Circle, Plasma, Waves, Terrain) with embedded GLSL fragment sources. Each shader includes shared functions (swirlUV, paletteColor, hash, bayer4x4, applyDither) via `concat!`. PresetControls struct with `has_angle`, `has_scale`, `has_speed`, `has_center`, `speed_label`, `speed_range`, `scale_range`. 583 lines.
- `/home/sbeam/code/wallpaper/src/window.rs` — Two-column layout: left (palette + pattern controls with blend/center hints), right (preview + effects with swirl/noise/dither + export). All UI construction and signal wiring. 841 lines.
- `/home/sbeam/code/wallpaper/src/shader.rs` — ShaderProgram compilation and linking. 65 lines.
- `/home/sbeam/code/wallpaper/src/export.rs` — Image export (PNG/JPEG at 1080p/1440p/4K). Auto-detects best default resolution from display. 119 lines.
- `/home/sbeam/code/wallpaper/src/wallpaper.rs` — GNOME wallpaper setting via gsettings (light + dark URIs). 34 lines.
- `/home/sbeam/code/wallpaper/install.sh` — Build + install script (release binary, desktop file, icon, metainfo, palettes to `~/.local` prefix). 57 lines.
- `/home/sbeam/code/wallpaper/data/palettes/` — Bundled palette PNGs in category subfolders (cold, dark, fall, gradient, light, pastel, retro, sunset, warm, winter). ~1,459 palette images total.
- `/home/sbeam/code/wallpaper/data/icons/com.megakode.Wallrus.svg` — App icon.
- `/home/sbeam/code/wallpaper/data/com.megakode.Wallrus.desktop` — Desktop entry.
- `/home/sbeam/code/wallpaper/data/com.megakode.Wallrus.metainfo.xml` — AppStream metadata.

## Architecture

```
main.rs
  └─ application.rs        AdwApplication lifecycle
       └─ window.rs         UI construction + signal wiring (841 lines, largest file)
            ├─ gl_renderer.rs   RendererState + GLArea creation + render/export
            │    └─ shader.rs       ShaderProgram compile/link
            ├─ shader_presets.rs    Preset names, controls, GLSL sources
            ├─ palette.rs           Palette image scanning + color extraction
            ├─ export.rs            PNG/JPEG file export
            └─ wallpaper.rs         gsettings wallpaper integration
```

### Uniform flow

All shader uniforms are stored as fields on `RendererState` in `gl_renderer.rs`. UI widgets in `window.rs` update these fields via signal handlers. The `render()` method uploads all uniforms every frame. Uniforms that don't exist in a particular shader are silently ignored (the `get_uniform_location` returns `None`).

| Uniform | Type | Range | Default | Used by |
|---------|------|-------|---------|---------|
| `uColor1-4` | vec3 | 0–1 RGB | preset defaults | all |
| `uAngle` | float | 0–2pi | pi/4 | Bars, Waves |
| `uScale` | float | per-preset | 1.0 | Circle, Plasma, Waves, Terrain |
| `uSpeed` | float | 0–20 | 0.0 | Plasma, Waves, Terrain |
| `uBlend` | float | 0–1 | 0.5 | all |
| `uSwirl` | float | -10–10 | 0.0 | all |
| `uNoise` | float | -1–1 | 0.0 | all |
| `uCenter` | float | -1–1 | 0.0 | Circle |
| `uDither` | float | 0 or 1 | 0.0 | all |
| `iResolution` | vec3 | viewport size | — | all |
| `iTime` | float | elapsed secs | — | all (unused in practice) |

### Shared GLSL functions (in every fragment shader)

- `swirlUV(vec2 uv)` — vortex UV distortion
- `paletteColor(float t)` — 4-band color lookup with blend control
- `hash(vec2 p)` — pseudo-random hash for noise grain
- `bayer4x4(vec2 p)` — 4x4 ordered dithering threshold
- `applyDither(vec3 color, vec2 fragCoord)` — conditional Bayer dithering

### Palette categories (bundled)

cold (200), dark (54), fall (200), gradient (200), light (200), pastel (200), retro (200), sunset (5), warm (200), winter (200).
