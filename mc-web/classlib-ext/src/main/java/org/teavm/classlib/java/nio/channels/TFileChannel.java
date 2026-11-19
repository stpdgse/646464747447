package org.teavm.classlib.java.nio.channels;

import java.io.IOException;
import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.nio.TByteBuffer;

/**
 * File channel contract. The browser build supplies a storage-backed implementation (OPFS) in a
 * later phase; until then {@code open} reports that clearly instead of failing at build time.
 */
public abstract class TFileChannel extends TObject implements TSeekableByteChannel {
    public static final class MapMode extends TObject {
        public static final MapMode READ_ONLY = new MapMode("READ_ONLY");
        public static final MapMode READ_WRITE = new MapMode("READ_WRITE");
        public static final MapMode PRIVATE = new MapMode("PRIVATE");

        private final String name;

        private MapMode(String name) {
            this.name = name;
        }

        @Override
        public String toString() {
            return name;
        }
    }

    protected TFileChannel() {
    }

    public static TFileChannel open(java.nio.file.Path path, java.nio.file.OpenOption... options)
            throws IOException {
        throw new UnsupportedOperationException("FileChannel needs the browser storage layer (not built yet)");
    }

    public static TFileChannel open(java.nio.file.Path path, java.util.Set<? extends java.nio.file.OpenOption> options,
            java.nio.file.attribute.FileAttribute<?>... attrs) throws IOException {
        throw new UnsupportedOperationException("FileChannel needs the browser storage layer (not built yet)");
    }

    @Override
    public abstract int read(TByteBuffer dst) throws IOException;

    public abstract int read(TByteBuffer dst, long position) throws IOException;

    @Override
    public abstract int write(TByteBuffer src) throws IOException;

    public abstract int write(TByteBuffer src, long position) throws IOException;

    @Override
    public abstract long position();

    @Override
    public abstract TFileChannel position(long newPosition) throws IOException;

    @Override
    public abstract long size() throws IOException;

    @Override
    public abstract TFileChannel truncate(long size) throws IOException;

    public abstract void force(boolean metaData) throws IOException;

    public abstract TFileLock lock(long position, long size, boolean shared) throws IOException;

    public abstract TFileLock tryLock(long position, long size, boolean shared) throws IOException;

    public final TFileLock tryLock() throws IOException {
        return tryLock(0L, Long.MAX_VALUE, false);
    }
}
