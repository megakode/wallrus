#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uCenter;

// common.glsl inserted here

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);

    // Offset center horizontally based on uCenter
    vec2 center = vec2(0.5 + uCenter * 0.4, 0.5);

    // Distance from offset center
    float d = length(uv - center);

    // Map distance to 0-1 across the visible radius, scaled
    float t = clamp(d * uScale, 0.0, 1.0);

    vec3 color = paletteColor(t);
    color = applyLighting(color, t, uv);

    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
