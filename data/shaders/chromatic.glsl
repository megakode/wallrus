#version 300 es
precision highp float;

uniform sampler2D uSceneTexture;
uniform vec3 iResolution;
uniform float uChromaticStrength;
uniform float uChromaticAngle;

out vec4 fragColor;

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;

    // Direction of the chromatic split
    vec2 dir = vec2(cos(uChromaticAngle), sin(uChromaticAngle));

    // Offset in UV space, scaled by strength (0..1 mapped to 0..~20 pixels)
    vec2 offset = dir * uChromaticStrength * 20.0 / iResolution.xy;

    // Sample each channel with a different offset
    float r = texture(uSceneTexture, clamp(uv + offset, 0.0, 1.0)).r;
    float g = texture(uSceneTexture, uv).g;
    float b = texture(uSceneTexture, clamp(uv - offset, 0.0, 1.0)).b;

    fragColor = vec4(r, g, b, 1.0);
}
