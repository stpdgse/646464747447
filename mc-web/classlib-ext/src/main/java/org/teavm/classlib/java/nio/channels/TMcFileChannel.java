package org.teavm.classlib.java.nio.channels;

import java.io.File;
import java.io.IOException;
import java.io.RandomAccessFile;
import org.teavm.classlib.java.nio.TByteBuffer;

/** A {@link TFileChannel} over a {@link RandomAccessFile} on the virtual file system. */
public final class TMcFileChannel extends TFileChannel {
    private final RandomAccessFile file;
    private boolean open = true;

    public TMcFileChannel(RandomAccessFile file) {
        this.file = file;
    }

    /** Opens a file with the usual {@code StandardOpenOption} semantics. */
    public static TMcFileChannel open(java.nio.file.Path path, java.util.Set<? extends java.nio.file.OpenOption> options)
            throws IOException {
        boolean write = options.contains(java.nio.file.StandardOpenOption.WRITE)
                || options.contains(java.nio.file.StandardOpenOption.APPEND)
                || options.contains(java.nio.file.StandardOpenOption.CREATE)
                || options.contains(java.nio.file.StandardOpenOption.CREATE_NEW)
                || options.contains(java.nio.file.StandardOpenOption.TRUNCATE_EXISTING);
        File f = new File(path.toString());
        if (options.contains(java.nio.file.StandardOpenOption.CREATE_NEW) && f.exists()) {
            throw new java.nio.file.FileAlreadyExistsException(path.toString());
        }
        boolean create = options.contains(java.nio.file.StandardOpenOption.CREATE)
                || options.contains(java.nio.file.StandardOpenOption.CREATE_NEW);
        if (!f.exists() && !create) {
            throw new java.nio.file.NoSuchFileException(path.toString());
        }
        if (!f.exists() && create) {
            File parent = f.getParentFile();
            if (parent != null && !parent.exists()) {
                parent.mkdirs();
            }
            f.createNewFile();
        }
        RandomAccessFile raf = new RandomAccessFile(f, write ? "rw" : "r");
        TMcFileChannel ch = new TMcFileChannel(raf);
        if (options.contains(java.nio.file.StandardOpenOption.TRUNCATE_EXISTING)) {
            raf.setLength(0);
        }
        if (options.contains(java.nio.file.StandardOpenOption.APPEND)) {
            raf.seek(raf.length());
        }
        return ch;
    }

    private void ensureOpen() throws IOException {
        if (!open) {
            throw new java.nio.channels.ClosedChannelException();
        }
    }

    @Override
    public int read(TByteBuffer dst) throws IOException {
        ensureOpen();
        // Use the real java.nio type inside bodies: naming TByteBuffer directly makes TeaVM load it twice.
        java.nio.ByteBuffer real = (java.nio.ByteBuffer) (Object) dst;
        int want = real.remaining();
        if (want == 0) {
            return 0;
        }
        byte[] tmp = new byte[Math.min(want, 65536)];
        int n = file.read(tmp, 0, tmp.length);
        if (n > 0) {
            real.put(tmp, 0, n);
        }
        return n;
    }

    @Override
    public int read(TByteBuffer dst, long position) throws IOException {
        long saved = file.getFilePointer();
        try {
            file.seek(position);
            return read(dst);
        } finally {
            file.seek(saved);
        }
    }

    @Override
    public int write(TByteBuffer src) throws IOException {
        ensureOpen();
        java.nio.ByteBuffer real = (java.nio.ByteBuffer) (Object) src;
        int n = real.remaining();
        byte[] tmp = new byte[n];
        real.get(tmp, 0, n);
        file.write(tmp, 0, n);
        return n;
    }

    @Override
    public int write(TByteBuffer src, long position) throws IOException {
        long saved = file.getFilePointer();
        try {
            file.seek(position);
            return write(src);
        } finally {
            file.seek(saved);
        }
    }

    @Override
    public long position() {
        try {
            return file.getFilePointer();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    @Override
    public TFileChannel position(long newPosition) throws IOException {
        ensureOpen();
        file.seek(newPosition);
        return this;
    }

    @Override
    public long size() throws IOException {
        ensureOpen();
        return file.length();
    }

    @Override
    public TFileChannel truncate(long size) throws IOException {
        ensureOpen();
        if (size < file.length()) {
            file.setLength(size);
        }
        return this;
    }

    @Override
    public void force(boolean metaData) {
    }

    @Override
    public TFileLock lock(long position, long size, boolean shared) {
        return new Lock();
    }

    @Override
    public TFileLock tryLock(long position, long size, boolean shared) {
        return new Lock();
    }

    @Override
    public boolean isOpen() {
        return open;
    }

    @Override
    public void close() throws IOException {
        if (open) {
            open = false;
            file.close();
        }
    }

    /** Single-process: every lock request succeeds. */
    private static final class Lock extends TFileLock {
        private boolean valid = true;

        @Override
        public boolean isValid() {
            return valid;
        }

        @Override
        public void release() {
            valid = false;
        }
    }
}
