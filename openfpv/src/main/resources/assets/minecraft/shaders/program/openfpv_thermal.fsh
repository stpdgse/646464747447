#version 120

// OpenFPV thermal vision: maps scene brightness onto an "iron" heat palette.
//  * Living things are drawn flat white by the mod, so they come out white-yellow (hot).
//  * Blue-dominant pixels (sky, water) are treated as cold, so they go deep purple/blue.
//  * Lit ground, lava and glowing blocks land in the orange/yellow part of the palette.

uniform sampler2D DiffuseSampler;
varying vec2 texCoord;

vec3 heat(float t) {
    t = clamp(t, 0.0, 1.0);
    vec3 c0 = vec3(0.02, 0.00, 0.10);
    vec3 c1 = vec3(0.30, 0.00, 0.45);
    vec3 c2 = vec3(0.75, 0.10, 0.20);
    vec3 c3 = vec3(0.98, 0.50, 0.05);
    vec3 c4 = vec3(1.00, 0.92, 0.35);
    vec3 c5 = vec3(1.00, 1.00, 1.00);
    if (t < 0.2)  return mix(c0, c1, t / 0.2);
    if (t < 0.4)  return mix(c1, c2, (t - 0.2) / 0.2);
    if (t < 0.6)  return mix(c2, c3, (t - 0.4) / 0.2);
    if (t < 0.8)  return mix(c3, c4, (t - 0.6) / 0.2);
    return mix(c4, c5, (t - 0.8) / 0.2);
}

void main() {
    vec3 c = texture2D(DiffuseSampler, texCoord).rgb;
    float l = dot(c, vec3(0.3, 0.59, 0.11));
    float cold = clamp((c.b - c.r) * 3.0, 0.0, 1.0);   // sky and water
    float t = pow(l, 0.8) * (1.0 - 0.85 * cold);
    gl_FragColor = vec4(heat(t), 1.0);
}
