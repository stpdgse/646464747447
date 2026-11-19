package org.teavm.classlib.java.util;

import org.teavm.classlib.java.lang.TComparable;
import org.teavm.classlib.java.lang.TObject;

/** Full 128-bit UUID (replaces TeaVM's string-backed placeholder). */
public final class TUUID extends TObject implements TComparable<TUUID> {
    private final long msb;
    private final long lsb;

    public TUUID(long mostSigBits, long leastSigBits) {
        this.msb = mostSigBits;
        this.lsb = leastSigBits;
    }

    private static TUUID fromBytes(byte[] d) {
        long m = 0;
        long l = 0;
        for (int i = 0; i < 8; i++) {
            m = (m << 8) | (d[i] & 0xff);
        }
        for (int i = 8; i < 16; i++) {
            l = (l << 8) | (d[i] & 0xff);
        }
        return new TUUID(m, l);
    }

    public static TUUID randomUUID() {
        byte[] b = new byte[16];
        for (int i = 0; i < 16; i++) {
            b[i] = (byte) (Math.random() * 256);
        }
        b[6] = (byte) ((b[6] & 0x0f) | 0x40);
        b[8] = (byte) ((b[8] & 0x3f) | 0x80);
        return fromBytes(b);
    }

    public static TUUID nameUUIDFromBytes(byte[] name) {
        byte[] md5 = org.teavm.classlib.java.security.TMessageDigest.digestOf("MD5", name);
        md5[6] = (byte) ((md5[6] & 0x0f) | 0x30);
        md5[8] = (byte) ((md5[8] & 0x3f) | 0x80);
        return fromBytes(md5);
    }

    public static TUUID fromString(String s) {
        String[] p = s.split("-");
        if (p.length != 5) {
            throw new IllegalArgumentException("Invalid UUID string: " + s);
        }
        long m = Long.parseLong(p[0], 16);
        m = (m << 16) | Long.parseLong(p[1], 16);
        m = (m << 16) | Long.parseLong(p[2], 16);
        long l = Long.parseLong(p[3], 16);
        l = (l << 48) | Long.parseLong(p[4], 16);
        return new TUUID(m, l);
    }

    public long getMostSignificantBits() {
        return msb;
    }

    public long getLeastSignificantBits() {
        return lsb;
    }

    public int version() {
        return (int) ((msb >> 12) & 0x0f);
    }

    public int variant() {
        return (int) ((lsb >>> (64 - (lsb >>> 62))) & (lsb >> 63));
    }

    private static String digits(long val, int digits) {
        long hi = 1L << (digits * 4);
        return Long.toHexString(hi | (val & (hi - 1))).substring(1);
    }

    @Override
    public String toString() {
        return digits(msb >> 32, 8) + "-" + digits(msb >> 16, 4) + "-" + digits(msb, 4) + "-"
                + digits(lsb >> 48, 4) + "-" + digits(lsb, 12);
    }

    @Override
    public int hashCode() {
        long x = msb ^ lsb;
        return ((int) (x >> 32)) ^ (int) x;
    }

    @Override
    public boolean equals(Object o) {
        return o instanceof TUUID && ((TUUID) o).msb == msb && ((TUUID) o).lsb == lsb;
    }

    @Override
    public int compareTo(TUUID o) {
        return msb < o.msb ? -1 : msb > o.msb ? 1 : lsb < o.lsb ? -1 : lsb > o.lsb ? 1 : 0;
    }
}
