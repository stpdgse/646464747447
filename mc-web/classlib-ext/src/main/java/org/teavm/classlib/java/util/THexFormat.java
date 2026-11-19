package org.teavm.classlib.java.util;

import org.teavm.classlib.java.lang.TObject;

public final class THexFormat extends TObject {
    private static final THexFormat DEFAULT = new THexFormat("", false);

    private final String prefix;
    private final boolean upper;

    private THexFormat(String prefix, boolean upper) {
        this.prefix = prefix;
        this.upper = upper;
    }

    public static THexFormat of() {
        return DEFAULT;
    }

    public THexFormat withPrefix(String p) {
        return new THexFormat(p, upper);
    }

    public THexFormat withUpperCase() {
        return new THexFormat(prefix, true);
    }

    public THexFormat withLowerCase() {
        return new THexFormat(prefix, false);
    }

    private char digit(int v) {
        char c = Character.forDigit(v & 0xf, 16);
        return upper ? Character.toUpperCase(c) : c;
    }

    public String formatHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder(bytes.length * (2 + prefix.length()));
        for (byte b : bytes) {
            sb.append(prefix).append(digit(b >> 4)).append(digit(b));
        }
        return sb.toString();
    }

    public String toHexDigits(byte v) {
        return "" + digit(v >> 4) + digit(v);
    }

    public String toHexDigits(int v) {
        return toHexDigits((long) v, 8);
    }

    public String toHexDigits(long v) {
        return toHexDigits(v, 16);
    }

    public String toHexDigits(long v, int digits) {
        char[] out = new char[digits];
        for (int i = digits - 1; i >= 0; i--) {
            out[i] = digit((int) (v & 0xf));
            v >>>= 4;
        }
        return new String(out);
    }

    public static int fromHexDigits(CharSequence s) {
        if (s.length() > 8) {
            throw new IllegalArgumentException("string length greater than 8: " + s.length());
        }
        return (int) fromHexDigitsToLong(s, 0, s.length());
    }

    public static long fromHexDigitsToLong(CharSequence s, int from, int to) {
        if (to - from > 16) {
            throw new IllegalArgumentException("string length greater than 16: " + (to - from));
        }
        long v = 0;
        for (int i = from; i < to; i++) {
            int d = Character.digit(s.charAt(i), 16);
            if (d < 0) {
                throw new NumberFormatException("not a hexadecimal digit: \"" + s.charAt(i) + "\"");
            }
            v = (v << 4) | d;
        }
        return v;
    }
}
