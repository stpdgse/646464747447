package org.teavm.mcweb.host;

/** Rewrites desktop GLSL 3.30 shader source into GLSL ES 3.00, which is what WebGL2 compiles. */
public final class GlslEs {
    private GlslEs() {
    }

    private static final String PRECISION = "precision highp float;\nprecision highp int;\n"
            + "precision highp sampler2D;\nprecision highp sampler3D;\nprecision highp samplerCube;\n"
            + "precision highp sampler2DArray;\nprecision highp isampler2D;\nprecision highp usampler2D;\n"
            + "precision highp sampler2DShadow;\n";

    public static String convert(String source) {
        StringBuilder out = new StringBuilder(source.length() + 256);
        boolean versionSeen = false;
        for (String line : source.split("\n", -1)) {
            String trimmed = line.trim();
            if (trimmed.startsWith("#version")) {
                out.append("#version 300 es\n").append(PRECISION);
                versionSeen = true;
                continue;
            }
            if (trimmed.startsWith("#extension")) {
                continue;
            }
            out.append(line).append('\n');
        }
        if (!versionSeen) {
            return "#version 300 es\n" + PRECISION + out;
        }
        return out.toString();
    }
}
