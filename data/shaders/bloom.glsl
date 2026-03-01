#version 300 es
precision highp float;

uniform sampler2D uSceneTexture;
uniform vec3 iResolution;
uniform float uBloomThreshold;
uniform float uBloomIntensity;

out vec4 fragColor;

// Gaussian weight
float gaussWeight(float offset, float sigma) {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    vec2 texelSize = 1.0 / iResolution.xy;

    vec3 original = texture(uSceneTexture, uv).rgb;

    // Extract bright areas above threshold
    float brightness = dot(original, vec3(0.2126, 0.7152, 0.0722));
    vec3 brightPass = max(original - vec3(uBloomThreshold), 0.0);
    // Scale based on how much above threshold
    float excess = max(brightness - uBloomThreshold, 0.0);
    brightPass *= excess / (excess + 0.5); // soft knee

    // Blur the bright pass with a 13x13 Gaussian kernel
    float radius = 24.0; // fixed blur radius for bloom
    float sigma = radius * 0.5;
    vec3 blurred = vec3(0.0);
    float totalWeight = 0.0;

    for (int x = -6; x <= 6; x++) {
        for (int y = -6; y <= 6; y++) {
            vec2 offset = vec2(float(x), float(y)) * texelSize * (radius / 6.0);
            vec2 sampleUv = clamp(uv + offset, 0.0, 1.0);

            vec3 sampleColor = texture(uSceneTexture, sampleUv).rgb;
            float sampleBrightness = dot(sampleColor, vec3(0.2126, 0.7152, 0.0722));
            vec3 sampleBright = max(sampleColor - vec3(uBloomThreshold), 0.0);
            float sampleExcess = max(sampleBrightness - uBloomThreshold, 0.0);
            sampleBright *= sampleExcess / (sampleExcess + 0.5);

            float dist = length(vec2(float(x), float(y)));
            float w = gaussWeight(dist, sigma / (radius / 6.0));
            blurred += sampleBright * w;
            totalWeight += w;
        }
    }

    blurred /= totalWeight;

    // Additive blend: original + bloom glow
    vec3 result = original + blurred * uBloomIntensity;

    fragColor = vec4(result, 1.0);
}
