#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uAngle;

// common.glsl inserted here

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);
    // Rotate the gradient direction
    vec2 dir = vec2(cos(uAngle), sin(uAngle));
    float t = dot(uv - 0.5, dir) + 0.5;
    t = clamp(t, 0.0, 1.0);
    vec3 color = paletteColor(t);
    color = applyLighting(color, t, uv);
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
