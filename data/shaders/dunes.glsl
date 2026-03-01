#version 300 es
precision highp float;
precision highp int;
uniform vec3 iResolution;
uniform float iTime;
uniform float uScale;
uniform float uSpeed;

// common.glsl inserted here

// Compute sine wave y-position for a dune layer
float duneWave(float x, float baseY, float freq, float phase, float amplitude) {
    return baseY + sin(x * freq + phase) * amplitude;
}

out vec4 fragColor;

void main() {
    vec2 uv = distortUV(gl_FragCoord.xy / iResolution.xy);

    float x = uv.x * uScale * 10.0 + uSpeed;
    float y = uv.y;

    // Stack of dune silhouettes, same frequency but phase-offset from each other
    float freq = 1.5;

    // Compute wave positions (top to bottom)
    float w0 = duneWave(x, 0.20, freq, 0.0,  0.07);
    float w1 = duneWave(x, 0.40, freq, 2.5,  0.08);
    float w2 = duneWave(x, 0.60, freq, 5.3,  0.07);
    float w3 = duneWave(x, 0.80, freq, 8.7,  0.075);

    // Thin edge to preserve sine shape faithfully
    float edge = 0.005;

    // Silhouette masks: 1.0 below wave, 0.0 above
    float below0 = smoothstep(w0 + edge, w0 - edge, y);
    float below1 = smoothstep(w1 + edge, w1 - edge, y);
    float below2 = smoothstep(w2 + edge, w2 - edge, y);
    float below3 = smoothstep(w3 + edge, w3 - edge, y);

    // Determine which band the pixel is in and compute a smooth t.
    //
    // UV y=0 is bottom, y=1 is top. w3 (baseY=0.80) is the highest wave,
    // w0 (baseY=0.20) is the lowest. belowN=1 means y < wN (pixel is
    // geometrically below wave N on screen).
    //
    // Check from highest wave down. If not below w3, pixel is in the sky
    // above all dunes. If below w3 but not w2, pixel is in the top band, etc.
    float t;
    if (below3 < 0.5) {
        // Above all waves: sky
        t = 0.0;
    } else if (below2 < 0.5) {
        // Between wave 3 (top) and wave 2
        float localT = (w3 - y) / max(w3 - w2, 0.001);
        localT = clamp(localT, 0.0, 1.0);
        t = mix(0.0, 0.25, localT);
    } else if (below1 < 0.5) {
        // Between wave 2 and wave 1
        float localT = (w2 - y) / max(w2 - w1, 0.001);
        localT = clamp(localT, 0.0, 1.0);
        t = mix(0.25, 0.50, localT);
    } else if (below0 < 0.5) {
        // Between wave 1 and wave 0
        float localT = (w1 - y) / max(w1 - w0, 0.001);
        localT = clamp(localT, 0.0, 1.0);
        t = mix(0.50, 0.75, localT);
    } else {
        // Below wave 0: foreground band (bottom of screen)
        float localT = smoothstep(w0, w0 - 0.20, y);
        t = mix(0.75, 1.0, localT);
    }

    vec3 color = paletteColor(t);
    color = applyLighting(color, t, uv);

    // Apply noise grain
    float n = hash(gl_FragCoord.xy);
    color += n * uNoise * 0.3;
    color = clamp(color, 0.0, 1.0);
    color = applyDither(color, gl_FragCoord.xy);
    fragColor = vec4(color, 1.0);
}
