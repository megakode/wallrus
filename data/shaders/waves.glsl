#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uAngle;
uniform float uScale;
uniform float uSpeed;

// common.glsl inserted here

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);
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
    color = applyLighting(color, t, uv);
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
