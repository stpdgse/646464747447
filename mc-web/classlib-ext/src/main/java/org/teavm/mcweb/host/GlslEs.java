package org.teavm.mcweb.host;

/** Rewrites desktop GLSL 3.30 shader source into GLSL ES 3.00, which is what WebGL2 compiles. */
public final class GlslEs {
    private GlslEs() {
    }

    private static final String PRECISION = "precision highp float;\nprecision highp int;\n"
            + "precision highp sampler2D;\nprecision highp sampler3D;\nprecision highp samplerCube;\n"
            + "precision highp sampler2DArray;\nprecision highp isampler2D;\nprecision highp usampler2D;\n"
            + "precision highp sampler2DShadow;\n";

    /**
     * Fragment outputs declared as arrays and written with run-time indices (the order-independent
     * transparency shaders) are illegal in GLSL ES: split them into one output per element and route
     * dynamic writes through generated setter functions.
     */
    private static String splitOutputArrays(String source) {
        java.util.regex.Pattern decl = java.util.regex.Pattern.compile(
                "layout\\(location = (\\d+)\\) out (\\w+) (\\w+)\\[(\\d+)\\];");
        java.util.regex.Matcher m = decl.matcher(source);
        StringBuilder out = new StringBuilder();
        int last = 0;
        java.util.List<String[]> arrays = new java.util.ArrayList<>();
        while (m.find()) {
            int location = Integer.parseInt(m.group(1));
            String type = m.group(2);
            String name = m.group(3);
            int size = Integer.parseInt(m.group(4));
            out.append(source, last, m.start());
            for (int i = 0; i < size; i++) {
                out.append("layout(location = ").append(location + i).append(") out ").append(type).append(' ')
                        .append(name).append('_').append(i).append(";\n");
            }
            arrays.add(new String[] {name, type, Integer.toString(size)});
            last = m.end();
        }
        if (arrays.isEmpty()) {
            return source;
        }
        out.append(source, last, source.length());
        String text = out.toString();
        for (String[] array : arrays) {
            text = rewriteArrayUses(text, array[0], array[1], Integer.parseInt(array[2]));
        }
        return text;
    }

    private static int matchBracket(String s, int open) {
        int depth = 0;
        for (int i = open; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '[') {
                depth++;
            } else if (c == ']' && --depth == 0) {
                return i;
            }
        }
        throw new IllegalStateException("unbalanced [ in shader");
    }

    private static String rewriteArrayUses(String text, String name, String type, int size) {
        StringBuilder out = new StringBuilder();
        int pos = 0;
        boolean needVectorSetter = false;
        boolean needComponentSetter = false;
        String marker = name + "[";
        while (true) {
            int at = text.indexOf(marker, pos);
            if (at < 0) {
                break;
            }
            // whole-word match only
            if (at > 0 && (Character.isLetterOrDigit(text.charAt(at - 1)) || text.charAt(at - 1) == '_')) {
                out.append(text, pos, at + marker.length());
                pos = at + marker.length();
                continue;
            }
            int firstClose = matchBracket(text, at + name.length());
            String index = text.substring(at + marker.length(), firstClose).trim();
            out.append(text, pos, at);
            boolean literal = index.matches("\\d+");
            if (firstClose + 1 < text.length() && text.charAt(firstClose + 1) == '[') {
                int secondClose = matchBracket(text, firstClose + 1);
                String component = text.substring(firstClose + 2, secondClose).trim();
                if (literal) {
                    out.append(name).append('_').append(index).append('[').append(component).append(']');
                    pos = secondClose + 1;
                    continue;
                }
                // dynamic element and component: must be an assignment statement
                int eq = secondClose + 1;
                while (Character.isWhitespace(text.charAt(eq))) {
                    eq++;
                }
                if (text.charAt(eq) != '=' || text.charAt(eq + 1) == '=') {
                    throw new IllegalStateException("unsupported dynamic read of fragment output " + name);
                }
                int end = text.indexOf(';', eq);
                String value = text.substring(eq + 1, end).trim();
                out.append(name).append("_setc(").append(index).append(", ").append(component).append(", ")
                        .append(value).append(")");
                needComponentSetter = true;
                pos = end;
            } else if (literal) {
                out.append(name).append('_').append(index);
                pos = firstClose + 1;
            } else {
                int eq = firstClose + 1;
                while (Character.isWhitespace(text.charAt(eq))) {
                    eq++;
                }
                if (text.charAt(eq) != '=' || text.charAt(eq + 1) == '=') {
                    throw new IllegalStateException("unsupported dynamic read of fragment output " + name);
                }
                int end = text.indexOf(';', eq);
                out.append(name).append("_set(").append(index).append(", ")
                        .append(text.substring(eq + 1, end).trim()).append(")");
                needVectorSetter = true;
                pos = end;
            }
        }
        out.append(text, pos, text.length());
        String result = out.toString();
        StringBuilder helpers = new StringBuilder();
        if (needVectorSetter) {
            helpers.append("void ").append(name).append("_set(int a, ").append(type).append(" v) {\n");
            for (int i = 0; i < size; i++) {
                helpers.append(i == 0 ? "    if" : "    else if").append(" (a == ").append(i).append(") ")
                        .append(name).append('_').append(i).append(" = v;\n");
            }
            helpers.append("}\n");
        }
        if (needComponentSetter) {
            // outputs cannot be indexed by a run-time component either: build the value in plain globals
            for (int i = 0; i < size; i++) {
                helpers.append(type).append(' ').append(name).append("_t_").append(i).append(" = ").append(type)
                        .append("(0.0);\n");
            }
            helpers.append("void ").append(name).append("_setc(int a, int c, float v) {\n");
            for (int i = 0; i < size; i++) {
                helpers.append(i == 0 ? "    if" : "    else if").append(" (a == ").append(i).append(") ")
                        .append(name).append("_t_").append(i).append("[c] = v;\n");
            }
            for (int i = 0; i < size; i++) {
                helpers.append("    ").append(name).append('_').append(i).append(" = ").append(name).append("_t_")
                        .append(i).append(";\n");
            }
            helpers.append("}\n");
        }
        if (helpers.length() > 0) {
            // helpers must come after the output declarations and before their first use: put them right
            // after the last "_<size-1>;" declaration
            String lastDecl = name + "_" + (size - 1) + ";\n";
            int declAt = result.indexOf(lastDecl);
            result = result.substring(0, declAt + lastDecl.length()) + helpers + result.substring(declAt + lastDecl.length());
        }
        return result;
    }

    /** Width (in texels) of the 2D textures that stand in for texture buffers; must match GL33C. */
    public static final int TEXEL_BUFFER_WIDTH = 2048;

    /**
     * WebGL2 has no texture buffers. They are emulated by 2D textures {@value #TEXEL_BUFFER_WIDTH} texels wide,
     * so the shader declares a 2D sampler and fetches with a computed 2D coordinate.
     */
    private static String emulateTexelBuffers(String source) {
        java.util.regex.Matcher m = java.util.regex.Pattern.compile("\\b([iu]?)samplerBuffer (\\w+);").matcher(source);
        java.util.List<String> names = new java.util.ArrayList<>();
        while (m.find()) {
            names.add(m.group(2));
        }
        if (names.isEmpty()) {
            return source;
        }
        String text = source.replaceAll("\\b([iu]?)samplerBuffer\\b", "$1sampler2D");
        for (String name : names) {
            String marker = "texelFetch(" + name + ",";
            StringBuilder out = new StringBuilder();
            int pos = 0;
            while (true) {
                int at = text.indexOf(marker, pos);
                if (at < 0) {
                    break;
                }
                int open = at + "texelFetch".length();
                int depth = 0;
                int close = -1;
                for (int i = open; i < text.length(); i++) {
                    char c = text.charAt(i);
                    if (c == '(') {
                        depth++;
                    } else if (c == ')' && --depth == 0) {
                        close = i;
                        break;
                    }
                }
                String index = text.substring(at + marker.length(), close).trim();
                out.append(text, pos, at).append("texelFetch(").append(name).append(", ivec2((").append(index)
                        .append(") % ").append(TEXEL_BUFFER_WIDTH).append(", (").append(index).append(") / ")
                        .append(TEXEL_BUFFER_WIDTH).append("), 0)");
                pos = close + 1;
            }
            out.append(text, pos, text.length());
            text = out.toString();
        }
        return text;
    }

    public static String convert(String source) {
        return convertLines(emulateTexelBuffers(splitOutputArrays(source)));
    }

    private static String convertLines(String source) {
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
