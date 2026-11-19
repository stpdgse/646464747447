package org.teavm.classlib.java.nio.channels;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.io.Reader;
import java.io.Writer;
import java.nio.charset.Charset;
import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.nio.TByteBuffer;

public final class TChannels extends TObject {
    private TChannels() {
    }

    public static TReadableByteChannel newChannel(InputStream in) {
        return new TReadableByteChannel() {
            private boolean open = true;

            @Override
            public int read(TByteBuffer dst) throws IOException {
                int n = dst.remaining();
                if (n == 0) {
                    return 0;
                }
                byte[] buf = new byte[Math.min(n, 8192)];
                int r = in.read(buf, 0, buf.length);
                if (r > 0) {
                    dst.put(buf, 0, r);
                }
                return r;
            }

            @Override
            public boolean isOpen() {
                return open;
            }

            @Override
            public void close() throws IOException {
                open = false;
                in.close();
            }
        };
    }

    public static TWritableByteChannel newChannel(OutputStream out) {
        return new TWritableByteChannel() {
            private boolean open = true;

            @Override
            public int write(TByteBuffer src) throws IOException {
                int n = src.remaining();
                byte[] buf = new byte[n];
                src.get(buf, 0, n);
                out.write(buf, 0, n);
                return n;
            }

            @Override
            public boolean isOpen() {
                return open;
            }

            @Override
            public void close() throws IOException {
                open = false;
                out.close();
            }
        };
    }

    public static InputStream newInputStream(TReadableByteChannel ch) {
        return new InputStream() {
            @Override
            public int read() throws IOException {
                byte[] one = new byte[1];
                int r = read(one, 0, 1);
                return r < 0 ? -1 : one[0] & 0xff;
            }

            @Override
            public int read(byte[] b, int off, int len) throws IOException {
                if (len == 0) {
                    return 0;
                }
                TByteBuffer buf = TByteBuffer.wrap(b, off, len);
                return ch.read(buf);
            }

            @Override
            public void close() throws IOException {
                ch.close();
            }
        };
    }

    public static OutputStream newOutputStream(TWritableByteChannel ch) {
        return new OutputStream() {
            @Override
            public void write(int b) throws IOException {
                write(new byte[] {(byte) b}, 0, 1);
            }

            @Override
            public void write(byte[] b, int off, int len) throws IOException {
                TByteBuffer buf = TByteBuffer.wrap(b, off, len);
                while (buf.hasRemaining()) {
                    ch.write(buf);
                }
            }

            @Override
            public void close() throws IOException {
                ch.close();
            }
        };
    }

    public static Reader newReader(TReadableByteChannel ch, Charset cs) {
        return new InputStreamReader(newInputStream(ch), cs);
    }

    public static Writer newWriter(TWritableByteChannel ch, Charset cs) {
        return new OutputStreamWriter(newOutputStream(ch), cs);
    }
}
