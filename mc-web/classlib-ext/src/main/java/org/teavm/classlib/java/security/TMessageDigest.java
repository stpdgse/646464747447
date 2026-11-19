package org.teavm.classlib.java.security;

import org.teavm.classlib.java.lang.TObject;

/**
 * Message digests implemented in plain Java: MD5, SHA-1, SHA-256 (what the game needs).
 * Written from the public FIPS 180-4 / RFC 1321 specifications.
 */
public abstract class TMessageDigest extends TObject {
    private final String algorithm;

    protected TMessageDigest(String algorithm) {
        this.algorithm = algorithm;
    }

    public static TMessageDigest getInstance(String algorithm) throws TNoSuchAlgorithmException {
        switch (algorithm.toUpperCase()) {
            case "MD5":
                return new Md("MD5", 16, false);
            case "SHA-1":
            case "SHA1":
                return new Md("SHA-1", 20, false);
            case "SHA-256":
            case "SHA256":
                return new Md("SHA-256", 32, false);
            default:
                throw new TNoSuchAlgorithmException(algorithm + " MessageDigest not available");
        }
    }

    public static byte[] digestOf(String algorithm, byte[] data) {
        try {
            TMessageDigest d = getInstance(algorithm);
            d.update(data, 0, data.length);
            return d.digest();
        } catch (TNoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    public String getAlgorithm() {
        return algorithm;
    }

    public abstract int getDigestLength();

    public abstract void update(byte input);

    public abstract void update(byte[] input, int offset, int len);

    public void update(byte[] input) {
        update(input, 0, input.length);
    }

    public void update(java.nio.ByteBuffer buf) {
        byte[] b = new byte[buf.remaining()];
        buf.get(b);
        update(b, 0, b.length);
    }

    public abstract byte[] digest();

    public byte[] digest(byte[] input) {
        update(input, 0, input.length);
        return digest();
    }

    public abstract void reset();

    public static boolean isEqual(byte[] a, byte[] b) {
        if (a == b) {
            return true;
        }
        if (a == null || b == null || a.length != b.length) {
            return false;
        }
        int r = 0;
        for (int i = 0; i < a.length; i++) {
            r |= a[i] ^ b[i];
        }
        return r == 0;
    }

    private static final class Md extends TMessageDigest {
        private final int kind; // 0 md5, 1 sha1, 2 sha256
        private final int length;
        private final byte[] block = new byte[64];
        private int blockLen;
        private long total;
        private int[] h;

        Md(String name, int length, boolean unused) {
            super(name);
            this.length = length;
            this.kind = length == 16 ? 0 : length == 20 ? 1 : 2;
            reset();
        }

        @Override
        public int getDigestLength() {
            return length;
        }

        @Override
        public void reset() {
            blockLen = 0;
            total = 0;
            switch (kind) {
                case 0:
                    h = new int[] {0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476};
                    break;
                case 1:
                    h = new int[] {0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0};
                    break;
                default:
                    h = new int[] {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
                    break;
            }
        }

        @Override
        public void update(byte input) {
            block[blockLen++] = input;
            total++;
            if (blockLen == 64) {
                process(block, 0);
                blockLen = 0;
            }
        }

        @Override
        public void update(byte[] in, int off, int len) {
            for (int i = 0; i < len; i++) {
                update(in[off + i]);
            }
        }

        @Override
        public byte[] digest() {
            long bits = total * 8;
            update((byte) 0x80);
            while (blockLen != 56) {
                update((byte) 0);
            }
            byte[] lenBytes = new byte[8];
            for (int i = 0; i < 8; i++) {
                int shift = kind == 0 ? i * 8 : (7 - i) * 8;
                lenBytes[i] = (byte) (bits >>> shift);
            }
            for (int i = 0; i < 8; i++) {
                block[blockLen++] = lenBytes[i];
            }
            process(block, 0);
            byte[] out = new byte[length];
            for (int i = 0; i < length; i++) {
                int word = h[i / 4];
                int shift = kind == 0 ? (i % 4) * 8 : (3 - i % 4) * 8;
                out[i] = (byte) (word >>> shift);
            }
            reset();
            return out;
        }

        private void process(byte[] b, int o) {
            switch (kind) {
                case 0:
                    md5(b, o);
                    break;
                case 1:
                    sha1(b, o);
                    break;
                default:
                    sha256(b, o);
                    break;
            }
        }

        private static int rol(int x, int n) {
            return (x << n) | (x >>> (32 - n));
        }

        private static int ror(int x, int n) {
            return (x >>> n) | (x << (32 - n));
        }

        private void sha1(byte[] b, int o) {
            int[] w = new int[80];
            for (int i = 0; i < 16; i++) {
                w[i] = ((b[o + i * 4] & 0xff) << 24) | ((b[o + i * 4 + 1] & 0xff) << 16)
                        | ((b[o + i * 4 + 2] & 0xff) << 8) | (b[o + i * 4 + 3] & 0xff);
            }
            for (int i = 16; i < 80; i++) {
                w[i] = rol(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
            }
            int a = h[0];
            int bb = h[1];
            int c = h[2];
            int d = h[3];
            int e = h[4];
            for (int i = 0; i < 80; i++) {
                int f;
                int k;
                if (i < 20) {
                    f = (bb & c) | (~bb & d);
                    k = 0x5a827999;
                } else if (i < 40) {
                    f = bb ^ c ^ d;
                    k = 0x6ed9eba1;
                } else if (i < 60) {
                    f = (bb & c) | (bb & d) | (c & d);
                    k = 0x8f1bbcdc;
                } else {
                    f = bb ^ c ^ d;
                    k = 0xca62c1d6;
                }
                int t = rol(a, 5) + f + e + k + w[i];
                e = d;
                d = c;
                c = rol(bb, 30);
                bb = a;
                a = t;
            }
            h[0] += a;
            h[1] += bb;
            h[2] += c;
            h[3] += d;
            h[4] += e;
        }

        private static final int[] K256 = {
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
        };

        private void sha256(byte[] b, int o) {
            int[] w = new int[64];
            for (int i = 0; i < 16; i++) {
                w[i] = ((b[o + i * 4] & 0xff) << 24) | ((b[o + i * 4 + 1] & 0xff) << 16)
                        | ((b[o + i * 4 + 2] & 0xff) << 8) | (b[o + i * 4 + 3] & 0xff);
            }
            for (int i = 16; i < 64; i++) {
                int s0 = ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^ (w[i - 15] >>> 3);
                int s1 = ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^ (w[i - 2] >>> 10);
                w[i] = w[i - 16] + s0 + w[i - 7] + s1;
            }
            int a = h[0];
            int bb = h[1];
            int c = h[2];
            int d = h[3];
            int e = h[4];
            int f = h[5];
            int g = h[6];
            int hh = h[7];
            for (int i = 0; i < 64; i++) {
                int s1 = ror(e, 6) ^ ror(e, 11) ^ ror(e, 25);
                int ch = (e & f) ^ (~e & g);
                int t1 = hh + s1 + ch + K256[i] + w[i];
                int s0 = ror(a, 2) ^ ror(a, 13) ^ ror(a, 22);
                int maj = (a & bb) ^ (a & c) ^ (bb & c);
                int t2 = s0 + maj;
                hh = g;
                g = f;
                f = e;
                e = d + t1;
                d = c;
                c = bb;
                bb = a;
                a = t1 + t2;
            }
            h[0] += a;
            h[1] += bb;
            h[2] += c;
            h[3] += d;
            h[4] += e;
            h[5] += f;
            h[6] += g;
            h[7] += hh;
        }

        private static final int[] S = {
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
            5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
            4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
        };

        private void md5(byte[] b, int o) {
            int[] m = new int[16];
            for (int i = 0; i < 16; i++) {
                m[i] = (b[o + i * 4] & 0xff) | ((b[o + i * 4 + 1] & 0xff) << 8)
                        | ((b[o + i * 4 + 2] & 0xff) << 16) | ((b[o + i * 4 + 3] & 0xff) << 24);
            }
            int a = h[0];
            int bb = h[1];
            int c = h[2];
            int d = h[3];
            for (int i = 0; i < 64; i++) {
                int f;
                int g;
                if (i < 16) {
                    f = (bb & c) | (~bb & d);
                    g = i;
                } else if (i < 32) {
                    f = (d & bb) | (~d & c);
                    g = (5 * i + 1) % 16;
                } else if (i < 48) {
                    f = bb ^ c ^ d;
                    g = (3 * i + 5) % 16;
                } else {
                    f = c ^ (bb | ~d);
                    g = (7 * i) % 16;
                }
                int k = (int) (long) Math.floor(Math.abs(Math.sin(i + 1)) * 4294967296.0);
                int tmp = d;
                d = c;
                c = bb;
                bb = bb + rol(a + f + k + m[g], S[i]);
                a = tmp;
            }
            h[0] += a;
            h[1] += bb;
            h[2] += c;
            h[3] += d;
        }
    }
}
