/// Shader preset definitions.
/// Each preset has a name, a fragment shader source, and a description of
/// which UI controls it needs.

/// Names of all available presets, in display order
pub fn preset_names() -> &'static [&'static str] {
    &["Bars", "Circle", "Plasma", "Waves", "Terrain"]
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
        "Bars" => Some(GRADIENT_FRAG.to_string()),
        "Plasma" => Some(PLASMA_FRAG.to_string()),
        "Waves" => Some(WAVES_FRAG.to_string()),
        "Terrain" => Some(TERRAIN_FRAG.to_string()),
        "Circle" => Some(CONCENTRIC_FRAG.to_string()),
        _ => None,
    }
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
            scale_range: (0.1, 5.0, 0.1, 1.0),
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
uniform float uNoise;
uniform float uDither;

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

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float bayer4x4(vec2 p) {
    ivec2 i = ivec2(p) & 3;
    int idx = i.x + i.y * 4;
    int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5);
    return float(b[idx]) / 16.0;
}

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
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
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
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
uniform float uNoise;
uniform float uDither;

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

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float bayer4x4(vec2 p) {
    ivec2 i = ivec2(p) & 3;
    int idx = i.x + i.y * 4;
    int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5);
    return float(b[idx]) / 16.0;
}

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
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
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
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
uniform float uNoise;
uniform float uDither;

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

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float bayer4x4(vec2 p) {
    ivec2 i = ivec2(p) & 3;
    int idx = i.x + i.y * 4;
    int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5);
    return float(b[idx]) / 16.0;
}

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
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
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
"#
);

const TERRAIN_FRAG: &str = concat!(
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
uniform float uNoise;
uniform float uDither;

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

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float bayer4x4(vec2 p) {
    ivec2 i = ivec2(p) & 3;
    int idx = i.x + i.y * 4;
    int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5);
    return float(b[idx]) / 16.0;
}

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
}

// Hash that returns a 2D gradient direction
vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)),
             dot(p, vec2(269.5, 183.3)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// 2D gradient noise — produces smooth rounded shapes
float gnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    // Quintic interpolation for extra smoothness
    vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    float a = dot(hash2(i + vec2(0.0, 0.0)), f - vec2(0.0, 0.0));
    float b = dot(hash2(i + vec2(1.0, 0.0)), f - vec2(1.0, 0.0));
    float c = dot(hash2(i + vec2(0.0, 1.0)), f - vec2(0.0, 1.0));
    float d = dot(hash2(i + vec2(1.0, 1.0)), f - vec2(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 0.5 + 0.5;
}

// Single octave of smooth gradient noise for very round hills
float fbm(vec2 p) {
    return gnoise(p);
}
"#,
    r#"
out vec4 fragColor;

void main() {
    vec2 uv = swirlUV(gl_FragCoord.xy / iResolution.xy);

    // Scale and offset the terrain
    vec2 p = (uv - 0.5) * uScale * 2.0;
    p += vec2(uSpeed * 1.7, uSpeed * 1.3);

    // Compute height field
    float height = fbm(p);
    // Remap to full 0–1 range
    height = clamp((height - 0.15) * 1.4, 0.0, 1.0);
    // Double smoothstep for extra-round contours
    height = height * height * (3.0 - 2.0 * height);
    height = height * height * (3.0 - 2.0 * height);

    // Map to palette
    vec3 color = paletteColor(height);

    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
"#
);

const CONCENTRIC_FRAG: &str = concat!(
    r#"#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uCenter;
"#,
    r#"
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uColor4;
uniform float uBlend;
uniform float uSwirl;
uniform float uNoise;
uniform float uDither;

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

float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float bayer4x4(vec2 p) {
    ivec2 i = ivec2(p) & 3;
    int idx = i.x + i.y * 4;
    int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5);
    return float(b[idx]) / 16.0;
}

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
}
"#,
    r#"
out vec4 fragColor;

void main() {
    vec2 uv = swirlUV(gl_FragCoord.xy / iResolution.xy);

    // Offset center horizontally based on uCenter
    vec2 center = vec2(0.5 + uCenter * 0.4, 0.5);

    // Distance from offset center
    float d = length(uv - center);

    // Map distance to 0-1 across the visible radius, scaled
    float t = clamp(d * uScale, 0.0, 1.0);

    vec3 color = paletteColor(t);

    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
"#
);
