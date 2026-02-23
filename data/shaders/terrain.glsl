#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uSpeed;

// common.glsl inserted here

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

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);

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
    color = applyLighting(color, height, uv);

    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
