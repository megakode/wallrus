#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uSpeed;

// common.glsl inserted here

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);
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
    color = applyLighting(color, t, uv);
    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
