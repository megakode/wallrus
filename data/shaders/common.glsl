
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uColor4;
uniform float uBlend;
uniform int uDistortType;
uniform float uDistortStrength;
uniform int uLightingType;
uniform float uLightStrength;
uniform float uBevelWidth;
uniform float uLightAngle;
uniform float uNoise;
uniform float uDither;

vec2 swirlUV(vec2 uv) {
    vec2 c = uv - 0.5;
    float r = length(c);
    float angle = uDistortStrength * (1.0 - r);
    float ca = cos(angle);
    float sa = sin(angle);
    return vec2(ca * c.x - sa * c.y, sa * c.x + ca * c.y) + 0.5;
}

vec2 fisheyeUV(vec2 uv) {
    vec2 c = uv - 0.5;
    float r = length(c);
    float power = 1.0 + uDistortStrength * 0.2;
    float bind = 0.5;
    // Barrel/pincushion distortion
    float nr = pow(r / bind, power) * bind;
    vec2 offset = (r > 0.0) ? c * (nr / r) : vec2(0.0);
    return offset + 0.5;
}

vec2 distortUV(vec2 uv) {
    if (uDistortType == 1) return swirlUV(uv);
    if (uDistortType == 2) return fisheyeUV(uv);
    return uv;
}

vec3 paletteColor(float t) {
    t = clamp(t, 0.0, 1.0);
    float fw = uBlend * 0.25;
    float f1 = (fw > 0.0001) ? smoothstep(0.25 - fw, 0.25 + fw, t) : step(0.25, t);
    float f2 = (fw > 0.0001) ? smoothstep(0.50 - fw, 0.50 + fw, t) : step(0.50, t);
    float f3 = (fw > 0.0001) ? smoothstep(0.75 - fw, 0.75 + fw, t) : step(0.75, t);
    vec3 color = uColor1;
    color = mix(color, uColor2, f1);
    color = mix(color, uColor3, f2);
    color = mix(color, uColor4, f3);
    return color;
}

vec3 applyLighting(vec3 color, float t, vec2 uv) {
    if (uLightingType == 0) return color;
    float shade = 0.0;
    if (uLightingType == 1) {
        // Bevel: shadow/highlight at color band boundaries
        float w = uBevelWidth;
        float b1 = smoothstep(-w, w, t - 0.25) * 2.0 - 1.0;
        float b2 = smoothstep(-w, w, t - 0.50) * 2.0 - 1.0;
        float b3 = smoothstep(-w, w, t - 0.75) * 2.0 - 1.0;
        float m1 = 1.0 - smoothstep(0.0, w * 2.0, abs(t - 0.25));
        float m2 = 1.0 - smoothstep(0.0, w * 2.0, abs(t - 0.50));
        float m3 = 1.0 - smoothstep(0.0, w * 2.0, abs(t - 0.75));
        shade = (b1 * m1 + b2 * m2 + b3 * m3) * 0.25;
    } else if (uLightingType == 2) {
        // Gradient: directional light across image (0 = from top)
        vec2 lightDir = vec2(cos(uLightAngle), sin(uLightAngle));
        shade = dot(uv - 0.5, lightDir);
    } else if (uLightingType == 3) {
        // Vignette: darken toward edges
        float dist = length(uv - 0.5) * 2.0;
        shade = -dist * 0.5;
    }
    return clamp(color + shade * uLightStrength, 0.0, 1.0);
}

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

vec3 applyDither(vec3 color, vec2 fragCoord) {
    if (uDither < 0.5) return color;
    float levels = 4.0;
    float threshold = bayer4x4(fragCoord) - 0.5;
    float step_ = 1.0 / levels;
    return floor(color / step_ + threshold + 0.5) * step_;
}
