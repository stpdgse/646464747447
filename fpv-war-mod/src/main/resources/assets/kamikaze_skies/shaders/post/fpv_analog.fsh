#version 330
#extension GL_ARB_separate_shader_objects : require

// Analog FPV video: scanlines, colour bleed, grain, rolling bar, horizontal tearing, and "snow" when the link is lost.
// All strengths come from the AnalogConfig block, chosen per signal level by scripts/gen_fpv_post.py.

uniform sampler2D InSampler;

layout(location = 0) in vec2 texCoord;

layout(std140) uniform SamplerInfo {
    vec2 OutSize;
    vec2 InSize;
};

layout(std140) uniform AnalogConfig {
    float Noise;    // grain strength
    float Scan;     // scanline darkening
    float Bleed;    // horizontal colour fringing, in UV units
    float Bar;      // rolling interference bar strength
    float Glitch;   // fraction of scanline bands torn sideways
    float Snow;     // blend toward pure static (1 = signal lost)
    float Seed;     // changes every variant so the noise animates
};

layout(location = 0) out vec4 fragColor;

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 uv = texCoord;

    // horizontal tearing: bands of the picture shift sideways
    float band = floor(uv.y * 36.0);
    if (hash(vec2(band, Seed)) < Glitch) {
        uv.x = fract(uv.x + (hash(vec2(band, Seed + 1.0)) - 0.5) * 0.14);
    }

    // rolling interference bar
    float barY = fract(Seed * 0.173);
    float d = uv.y - barY;
    float inBar = smoothstep(0.0, 0.03, d) * (1.0 - smoothstep(0.07, 0.11, d));
    uv.x += inBar * Bar * 0.02 * sin(uv.y * 220.0 + Seed);

    // colour bleed: red and blue sampled slightly apart
    float r = texture(InSampler, uv + vec2(Bleed, 0.0)).r;
    float g = texture(InSampler, uv).g;
    float b = texture(InSampler, uv - vec2(Bleed, 0.0)).b;
    vec3 col = vec3(r, g, b);

    // slightly washed-out analog colour
    float luma = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(luma), col, 0.88);
    col += inBar * Bar * 0.22;

    // scanlines
    float line = 0.5 + 0.5 * sin(texCoord.y * OutSize.y * 3.14159265);
    col *= 1.0 - Scan * (1.0 - line);

    // grain
    float n = hash(texCoord * OutSize + Seed * 17.0);
    col += (n - 0.5) * Noise;

    // signal lost: static
    col = mix(col, vec3(hash(texCoord * OutSize + Seed * 31.0)), Snow);

    // vignette (lens)
    vec2 q = texCoord - 0.5;
    col *= 1.0 - dot(q, q) * 0.55;

    fragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
