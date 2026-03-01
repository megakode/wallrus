/// Shader preset definitions.
/// Each preset has a name, a fragment shader source, and a description of
/// which UI controls it needs.

/// The shared common GLSL code (uniforms, utility functions) included in all
/// fragment shaders at the `// common.glsl inserted here` marker.
const COMMON_GLSL: &str = include_str!("../data/shaders/common.glsl");

/// Names of all available presets, in display order
pub fn preset_names() -> &'static [&'static str] {
    &["Bars", "Circle", "Plasma", "Waves", "Terrain"]
}

/// Returns the shared vertex shader source (fullscreen quad passthrough)
pub fn vertex_shader_source() -> String {
    include_str!("../data/shaders/vertex.glsl").to_string()
}

/// Assemble a fragment shader by inserting the common code at the marker.
fn assemble(shader_src: &str) -> String {
    shader_src.replacen("// common.glsl inserted here", COMMON_GLSL, 1)
}

/// Returns the fragment shader source for a given preset name
pub fn fragment_source_for(name: &str) -> Option<String> {
    let src = match name {
        "Bars" => include_str!("../data/shaders/bars.glsl"),
        "Plasma" => include_str!("../data/shaders/plasma.glsl"),
        "Waves" => include_str!("../data/shaders/waves.glsl"),
        "Terrain" => include_str!("../data/shaders/terrain.glsl"),
        "Circle" => include_str!("../data/shaders/circle.glsl"),
        _ => return None,
    };
    Some(assemble(src))
}

/// Returns the blur post-processing fragment shader source
pub fn blur_fragment_source() -> String {
    include_str!("../data/shaders/blur.glsl").to_string()
}

/// Returns the bloom/glow post-processing fragment shader source
pub fn bloom_fragment_source() -> String {
    include_str!("../data/shaders/bloom.glsl").to_string()
}

/// Returns the chromatic aberration post-processing fragment shader source
pub fn chromatic_fragment_source() -> String {
    include_str!("../data/shaders/chromatic.glsl").to_string()
}

/// Which UI controls a preset uses
pub struct PresetControls {
    pub has_angle: bool,
    pub has_scale: bool,
    pub has_speed: bool,
    pub has_center: bool,
    /// Label for the speed/time slider (e.g. "Speed" or "Time")
    pub speed_label: &'static str,
    /// Range for the speed/time slider: (min, max, step, default)
    pub speed_range: (f64, f64, f64, f64),
    /// Range for the scale slider: (min, max, step, default)
    pub scale_range: (f64, f64, f64, f64),
}

pub fn controls_for(name: &str) -> PresetControls {
    match name {
        "Bars" => PresetControls {
            has_angle: true,
            has_scale: false,
            has_speed: false,
            has_center: false,
            speed_label: "Speed",
            speed_range: (0.0, 3.0, 0.1, 1.0),
            scale_range: (0.1, 5.0, 0.1, 1.0),
        },
        "Plasma" => PresetControls {
            has_angle: false,
            has_scale: true,
            has_speed: true,
            has_center: false,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
            scale_range: (0.1, 2.0, 0.1, 1.0),
        },
        "Waves" => PresetControls {
            has_angle: true,
            has_scale: true,
            has_speed: true,
            has_center: false,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
            scale_range: (0.1, 5.0, 0.1, 1.0),
        },
        "Terrain" => PresetControls {
            has_angle: false,
            has_scale: true,
            has_speed: true,
            has_center: false,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
            scale_range: (0.1, 2.0, 0.01, 0.5),
        },
        "Circle" => PresetControls {
            has_angle: false,
            has_scale: true,
            has_speed: false,
            has_center: true,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
            scale_range: (0.5, 3.0, 0.1, 1.0),
        },
        _ => PresetControls {
            has_angle: true,
            has_scale: false,
            has_speed: false,
            has_center: false,
            speed_label: "Speed",
            speed_range: (0.0, 3.0, 0.1, 1.0),
            scale_range: (0.1, 5.0, 0.1, 1.0),
        },
    }
}
