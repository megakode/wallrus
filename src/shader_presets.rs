/// Shader preset definitions.
/// Each preset has a name, a fragment shader source, and a description of
/// which UI controls it needs.

/// Names of all available presets, in display order
pub fn preset_names() -> &'static [&'static str] {
    &["Gradient", "Plasma", "Waves"]
}

/// Returns the shared vertex shader source (fullscreen quad passthrough)
pub fn vertex_shader_source() -> String {
    String::from(
        "#version 330 core
layout(location = 0) in vec2 aPos;
void main() {
    gl_Position = vec4(aPos, 0.0, 1.0);
}
",
    )
}

/// Returns the fragment shader source for a given preset name
pub fn fragment_source_for(name: &str) -> Option<String> {
    match name {
        "Gradient" => Some(GRADIENT_FRAG.to_string()),
        "Plasma" => Some(PLASMA_FRAG.to_string()),
        "Waves" => Some(WAVES_FRAG.to_string()),
        _ => None,
    }
}

/// Which UI controls a preset uses
pub struct PresetControls {
    pub has_angle: bool,
    pub has_scale: bool,
    pub has_speed: bool,
    /// Label for the speed/time slider (e.g. "Speed" or "Time")
    pub speed_label: &'static str,
    /// Range for the speed/time slider: (min, max, step, default)
    pub speed_range: (f64, f64, f64, f64),
}

pub fn controls_for(name: &str) -> PresetControls {
    match name {
        "Gradient" => PresetControls {
            has_angle: true,
            has_scale: false,
            has_speed: false,
            speed_label: "Speed",
            speed_range: (0.0, 3.0, 0.1, 1.0),
        },
        "Plasma" => PresetControls {
            has_angle: false,
            has_scale: true,
            has_speed: true,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
        },
        "Waves" => PresetControls {
            has_angle: true,
            has_scale: true,
            has_speed: true,
            speed_label: "Time",
            speed_range: (0.0, 20.0, 0.1, 0.0),
        },
        _ => PresetControls {
            has_angle: true,
            has_scale: false,
            has_speed: false,
            speed_label: "Speed",
            speed_range: (0.0, 3.0, 0.1, 1.0),
        },
    }
}

// ---------------------------------------------------------------------------
// Fragment shader sources (embedded)
// ---------------------------------------------------------------------------

const GRADIENT_FRAG: &str = concat!(
    r#"#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uAngle;
"#,
    r#"
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uColor4;
uniform float uBlend;
uniform float uSwirl;

vec2 swirlUV(vec2 uv) {
    vec2 c = uv - 0.5;
    float r = length(c);
    float angle = uSwirl * (1.0 - r);
    float ca = cos(angle);
    float sa = sin(angle);
    return vec2(ca * c.x - sa * c.y, sa * c.x + ca * c.y) + 0.5;
}

vec3 paletteColor(float t) {
    t = clamp(t, 0.0, 1.0);
    // 4 equal bands: boundaries at 0.25, 0.5, 0.75
    // uBlend 0 = hard edges, 1 = fully smooth
    // Fade half-width: 0 at blend=0, 0.25 at blend=1
    float fw = uBlend * 0.25;
    // Compute blend factors between adjacent color pairs
    float f1 = (fw > 0.0001) ? smoothstep(0.25 - fw, 0.25 + fw, t) : step(0.25, t);
    float f2 = (fw > 0.0001) ? smoothstep(0.50 - fw, 0.50 + fw, t) : step(0.50, t);
    float f3 = (fw > 0.0001) ? smoothstep(0.75 - fw, 0.75 + fw, t) : step(0.75, t);
    vec3 color = uColor1;
    color = mix(color, uColor2, f1);
    color = mix(color, uColor3, f2);
    color = mix(color, uColor4, f3);
    return color;
}
"#,
    r#"
out vec4 fragColor;

void main() {
    vec2 uv = swirlUV(gl_FragCoord.xy / iResolution.xy);
    // Rotate the gradient direction
    vec2 dir = vec2(cos(uAngle), sin(uAngle));
    float t = dot(uv - 0.5, dir) + 0.5;
    t = clamp(t, 0.0, 1.0);
    vec3 color = paletteColor(t);
    fragColor = vec4(color, 1.0);
}
"#
);

const PLASMA_FRAG: &str = concat!(
    r#"#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uSpeed;
"#,
    r#"
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uColor4;
uniform float uBlend;
uniform float uSwirl;

vec2 swirlUV(vec2 uv) {
    vec2 c = uv - 0.5;
    float r = length(c);
    float angle = uSwirl * (1.0 - r);
    float ca = cos(angle);
    float sa = sin(angle);
    return vec2(ca * c.x - sa * c.y, sa * c.x + ca * c.y) + 0.5;
}

vec3 paletteColor(float t) {
    t = clamp(t, 0.0, 1.0);
    float fw = uBlend * 0.25;
    float f1 = (fw > 0.0001) ? smoothstep(0.25 - fw, 0.25 + fw, t) : step(0.25, t);
    float f2 = (fw > 0.0001) ? smoothstep(0.50 - fw, 0.50 + fw, t) : step(0.50, t);
    float f3 = (fw > 0.0001) ? smoothstep(0.75 - fw, 0.75 + fw, t) : step(0.75, t);
    vec3 color = uColor1;
    color = mix(color, uColor2, f1);
    color = mix(color, uColor3, f2);
    color = mix(color, uColor4, f3);
    return color;
}
"#,
    r#"
out vec4 fragColor;

void main() {
    vec2 uv = swirlUV(gl_FragCoord.xy / iResolution.xy);
    float time = uSpeed;

    float v = 0.0;
    vec2 p = (uv - 0.5) * uScale * 10.0;

    v += sin(p.x + time);
    v += sin((p.y + time) * 0.5);
    v += sin((p.x + p.y + time) * 0.5);

    float cx = p.x + 0.5 * sin(time * 0.33);
    float cy = p.y + 0.5 * cos(time * 0.5);
    v += sin(sqrt(cx * cx + cy * cy + 1.0) + time);

    v = v * 0.5;
    float t = sin(v * 3.14159) * 0.5 + 0.5;

    vec3 color = paletteColor(t);
    fragColor = vec4(color, 1.0);
}
"#
);

const WAVES_FRAG: &str = concat!(
    r#"#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uAngle;
uniform float uScale;
uniform float uSpeed;
"#,
    r#"
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uColor4;
uniform float uBlend;
uniform float uSwirl;

vec2 swirlUV(vec2 uv) {
    vec2 c = uv - 0.5;
    float r = length(c);
    float angle = uSwirl * (1.0 - r);
    float ca = cos(angle);
    float sa = sin(angle);
    return vec2(ca * c.x - sa * c.y, sa * c.x + ca * c.y) + 0.5;
}

vec3 paletteColor(float t) {
    t = clamp(t, 0.0, 1.0);
    float fw = uBlend * 0.25;
    float f1 = (fw > 0.0001) ? smoothstep(0.25 - fw, 0.25 + fw, t) : step(0.25, t);
    float f2 = (fw > 0.0001) ? smoothstep(0.50 - fw, 0.50 + fw, t) : step(0.50, t);
    float f3 = (fw > 0.0001) ? smoothstep(0.75 - fw, 0.75 + fw, t) : step(0.75, t);
    vec3 color = uColor1;
    color = mix(color, uColor2, f1);
    color = mix(color, uColor3, f2);
    color = mix(color, uColor4, f3);
    return color;
}
"#,
    r#"
out vec4 fragColor;

void main() {
    vec2 uv = swirlUV(gl_FragCoord.xy / iResolution.xy);
    float time = uSpeed;

    // Rotate UV
    vec2 center = uv - 0.5;
    float ca = cos(uAngle);
    float sa = sin(uAngle);
    vec2 ruv = vec2(ca * center.x - sa * center.y,
                    sa * center.x + ca * center.y);
    ruv += 0.5;

    // Layered sine waves
    float wave = 0.0;
    wave += sin(ruv.x * uScale * 20.0 + time * 2.0) * 0.25;
    wave += sin(ruv.x * uScale * 10.0 - time * 1.5 + ruv.y * 5.0) * 0.25;
    wave += sin(ruv.y * uScale * 15.0 + time * 1.0) * 0.15;
    wave += sin(length(center) * uScale * 15.0 - time * 2.0) * 0.2;

    float t = wave + ruv.y;
    t = clamp(t, 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t);

    vec3 color = paletteColor(t);
    fragColor = vec4(color, 1.0);
}
"#
);
