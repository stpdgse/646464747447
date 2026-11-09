#version 120

// OpenFPV night vision: green phosphor look. A soft-knee curve boosts dark areas strongly
// but keeps bright daytime areas from clipping to flat green.

uniform sampler2D DiffuseSampler;
varying vec2 texCoord;

void main() {
    vec3 c = texture2D(DiffuseSampler, texCoord).rgb;
    float l = dot(c, vec3(0.3, 0.59, 0.11));
    l = 1.0 - exp(-2.4 * l);
    gl_FragColor = vec4(l * 0.30, l, l * 0.38, 1.0);
}
