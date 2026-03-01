#version 300 es
precision highp float;

uniform sampler2D uSceneTexture;
uniform vec3 iResolution;
uniform int uBlurType;
uniform float uBlurStrength;
uniform float uBlurAngle;

out vec4 fragColor;

// Gaussian weight for a given offset distance
float gaussWeight(float offset, float sigma) {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

// Sample the scene texture with clamped coordinates
vec3 sampleScene(vec2 uv) {
    return texture(uSceneTexture, clamp(uv, 0.0, 1.0)).rgb;
}

// --- Gaussian blur: uniform disc/kernel blur ---
vec3 gaussianBlur(vec2 uv, vec2 texelSize) {
    float radius = uBlurStrength * 48.0;
    float sigma = max(radius * 0.5, 0.001);
    vec3 color = vec3(0.0);
    float totalWeight = 0.0;

    // 13x13 kernel
    for (int x = -6; x <= 6; x++) {
        for (int y = -6; y <= 6; y++) {
            vec2 offset = vec2(float(x), float(y)) * texelSize * (radius / 6.0);
            float dist = length(vec2(float(x), float(y)));
            float w = gaussWeight(dist, sigma / (radius / 6.0));
            color += sampleScene(uv + offset) * w;
            totalWeight += w;
        }
    }

    return color / totalWeight;
}

// --- Tilt-Shift blur: sharp band in the middle, blurred above/below ---
vec3 tiltShiftBlur(vec2 uv, vec2 texelSize) {
    float radius = uBlurStrength * 48.0;
    float sigma = max(radius * 0.5, 0.001);

    // Compute distance from the center band along the blur angle direction
    float angle = uBlurAngle;
    vec2 dir = vec2(-sin(angle), cos(angle));
    float distFromBand = abs(dot(uv - 0.5, dir));

    // Smooth falloff: sharp in center band, blurred toward edges
    float bandWidth = 0.15;
    float blurFactor = smoothstep(bandWidth, bandWidth + 0.3, distFromBand);

    if (blurFactor < 0.01) {
        return sampleScene(uv);
    }

    float effectiveRadius = radius * blurFactor;
    float effectiveSigma = max(effectiveRadius * 0.5, 0.001);

    vec3 color = vec3(0.0);
    float totalWeight = 0.0;

    for (int x = -6; x <= 6; x++) {
        for (int y = -6; y <= 6; y++) {
            vec2 offset = vec2(float(x), float(y)) * texelSize * (effectiveRadius / 6.0);
            float dist = length(vec2(float(x), float(y)));
            float w = gaussWeight(dist, effectiveSigma / (effectiveRadius / 6.0));
            color += sampleScene(uv + offset) * w;
            totalWeight += w;
        }
    }

    return color / totalWeight;
}

// --- Radial blur: samples along rays from center, stronger at edges ---
vec3 radialBlur(vec2 uv, vec2 texelSize) {
    vec2 center = vec2(0.5, 0.5);
    vec2 toCenter = uv - center;
    float dist = length(toCenter);

    // Blur strength increases with distance from center
    float blurAmount = uBlurStrength * 0.2 * dist;

    vec3 color = vec3(0.0);
    float totalWeight = 0.0;
    int samples = 25;

    for (int i = 0; i < 25; i++) {
        float t = (float(i) / float(samples - 1)) - 0.5; // -0.5 to 0.5
        vec2 offset = toCenter * t * blurAmount;
        float w = gaussWeight(t, 0.35);
        color += sampleScene(uv + offset) * w;
        totalWeight += w;
    }

    return color / totalWeight;
}

// --- Vignette blur: blur increases radially from center ---
vec3 vignetteBlur(vec2 uv, vec2 texelSize) {
    float radius = uBlurStrength * 48.0;

    // Radial distance from center
    float dist = length(uv - 0.5) * 2.0; // 0 at center, ~1.4 at corners
    float blurFactor = smoothstep(0.2, 1.2, dist);

    if (blurFactor < 0.01) {
        return sampleScene(uv);
    }

    float effectiveRadius = radius * blurFactor;
    float effectiveSigma = max(effectiveRadius * 0.5, 0.001);

    vec3 color = vec3(0.0);
    float totalWeight = 0.0;

    for (int x = -6; x <= 6; x++) {
        for (int y = -6; y <= 6; y++) {
            vec2 offset = vec2(float(x), float(y)) * texelSize * (effectiveRadius / 6.0);
            float dist2 = length(vec2(float(x), float(y)));
            float w = gaussWeight(dist2, effectiveSigma / max(effectiveRadius / 6.0, 0.001));
            color += sampleScene(uv + offset) * w;
            totalWeight += w;
        }
    }

    return color / totalWeight;
}

// --- Directional / motion blur: blur along a direction ---
vec3 directionalBlur(vec2 uv, vec2 texelSize) {
    float radius = uBlurStrength * 200.0;
    vec2 dir = vec2(cos(uBlurAngle), sin(uBlurAngle)) * texelSize * radius;

    vec3 color = vec3(0.0);
    float totalWeight = 0.0;
    int samples = 37;

    for (int i = 0; i < 37; i++) {
        float t = (float(i) / float(samples - 1)) - 0.5; // -0.5 to 0.5
        vec2 offset = dir * t;
        float w = gaussWeight(t, 0.35);
        color += sampleScene(uv + offset) * w;
        totalWeight += w;
    }

    return color / totalWeight;
}

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    vec2 texelSize = 1.0 / iResolution.xy;

    vec3 color;

    if (uBlurType == 1) {
        color = gaussianBlur(uv, texelSize);
    } else if (uBlurType == 2) {
        color = tiltShiftBlur(uv, texelSize);
    } else if (uBlurType == 3) {
        color = radialBlur(uv, texelSize);
    } else if (uBlurType == 4) {
        color = vignetteBlur(uv, texelSize);
    } else if (uBlurType == 5) {
        color = directionalBlur(uv, texelSize);
    } else {
        color = sampleScene(uv);
    }

    fragColor = vec4(color, 1.0);
}
