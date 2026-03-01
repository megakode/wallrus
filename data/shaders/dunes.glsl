#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uSpeed;

// common.glsl inserted here

// Each dune layer: a sine wave at a given vertical position
// Returns a smooth gradient: 1.0 well below the wave, 0.0 well above
float duneLayer(float x, float y, float baseY, float freq, float phase, float amplitude) {
    float wave = baseY + sin(x * freq + phase) * amplitude;
    // Wide soft edge so palette transitions are smooth
    return smoothstep(wave + 0.08, wave - 0.08, y);
}

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);

    float x = uv.x * uScale * 10.0 + uSpeed;
    float y = uv.y;

    // Stack of dune silhouettes, same frequency but phase-offset from each other
    float freq = 1.5;
    // Layer 0: background (top)
    float l0 = duneLayer(x, y, 0.20, freq, 0.0, 0.03);
    // Layer 1
    float l1 = duneLayer(x, y, 0.40, freq, 2.5, 0.04);
    // Layer 2
    float l2 = duneLayer(x, y, 0.60, freq, 5.3, 0.03);
    // Layer 3: foreground (bottom)
    float l3 = duneLayer(x, y, 0.80, freq, 8.7, 0.035);

    // Combine: each layer advances through the palette
    float t = l0 * 0.25 + l1 * 0.25 + l2 * 0.25 + l3 * 0.25;
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
