#version 120

// OpenFPV night vision: green phosphor look with a little amplification.

uniform sampler2D DiffuseSampler;
varying vec2 texCoord;

void main() {
    vec3 c = texture2D(DiffuseSampler, texCoord).rgb;
    float l = dot(c, vec3(0.3, 0.59, 0.11));
    l = clamp(pow(l, 0.8) * 1.35, 0.0, 1.0);
    gl_FragColor = vec4(l * 0.30, l, l * 0.38, 1.0);
}
