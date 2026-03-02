#version 300 es
precision highp float;
precision highp int;

uniform sampler2D uSceneTexture;
uniform vec3 iResolution;
uniform int uEffectType;      // 0=none, 1=noise, 2=dither, 3=film grain
uniform float uNoise;
uniform float uDither;
uniform float uDitherStrength;
uniform float uDitherLevels;
uniform float uGrainIntensity;

out vec4 fragColor;

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

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    vec3 color = texture(uSceneTexture, uv).rgb;

    if (uEffectType == 1) {
        // Noise grain
        float n = hash(gl_FragCoord.xy);
        color += n * uNoise * 0.3;
        color = clamp(color, 0.0, 1.0);
    } else if (uEffectType == 2) {
        // Ordered dither
        float threshold = (bayer4x4(gl_FragCoord.xy) - 0.5) * uDitherStrength;
        float step_ = 1.0 / uDitherLevels;
        color = clamp(floor(color / step_ + threshold + 0.5) * step_, 0.0, 1.0);
    } else if (uEffectType == 3) {
        // Film grain — luminance-dependent
        // More grain in midtones, less in highlights and shadows
        float n = hash(gl_FragCoord.xy) * 2.0 - 1.0; // -1..1

        float luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
        // Bell curve: peaks at luminance=0.5, fades at 0 and 1
        float midtoneMask = 1.0 - pow(2.0 * luminance - 1.0, 2.0);

        color += n * uGrainIntensity * midtoneMask;
        color = clamp(color, 0.0, 1.0);
    }

    fragColor = vec4(color, 1.0);
}
