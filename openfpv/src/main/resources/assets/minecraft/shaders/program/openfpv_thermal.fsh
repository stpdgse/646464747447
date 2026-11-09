#version 120

// OpenFPV thermal vision: maps scene brightness onto an "iron" heat palette.
// Entities are drawn flat white (hot) by the mod, so they come out white-yellow;
// cold dark scenery ends up purple/blue, sunlit and lava surfaces glow orange.

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
    l = pow(l, 0.75);
    gl_FragColor = vec4(heat(l), 1.0);
}
