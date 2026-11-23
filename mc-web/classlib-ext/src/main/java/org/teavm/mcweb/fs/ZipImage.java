package org.teavm.mcweb.fs;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import java.util.zip.DataFormatException;
import java.util.zip.Inflater;

/** An in-memory zip archive: parsed central directory plus lazy per-entry inflation. */
public final class ZipImage {
    /** One archive entry (file) or an implied/explicit directory. */
    public static final class Node {
        final String name;
        final boolean directory;
        final int method;
        final long compressedSize;
        final long size;
        final long dataOffset;
        final long modified;
        final TreeSet<String> children = new TreeSet<>();

        Node(String name, boolean directory, int method, long compressedSize, long size, long dataOffset,
                long modified) {
            this.name = name;
            this.directory = directory;
            this.method = method;
            this.compressedSize = compressedSize;
            this.size = size;
            this.dataOffset = dataOffset;
            this.modified = modified;
        }

        public boolean isDirectory() {
            return directory;
        }

        public long size() {
            return size;
        }

        public long lastModified() {
            return modified;
        }
    }

    private final byte[] data;
    private final Map<String, Node> nodes = new HashMap<>();

    public ZipImage(byte[] data) throws IOException {
        this.data = data;
        nodes.put("/", new Node("/", true, 0, 0, 0, 0, 0));
        parse();
    }

    private int u16(int p) {
        return (data[p] & 0xff) | ((data[p + 1] & 0xff) << 8);
    }

    private long u32(int p) {
        return (u16(p) & 0xffffL) | ((long) u16(p + 2) << 16);
    }

    private void parse() throws IOException {
        int eocd = -1;
        for (int p = data.length - 22; p >= Math.max(0, data.length - 22 - 65535); p--) {
            if (u32(p) == 0x06054b50L) {
                eocd = p;
                break;
            }
        }
        if (eocd < 0) {
            throw new IOException("not a zip archive");
        }
        int count = u16(eocd + 10);
        int p = (int) u32(eocd + 16);
        for (int i = 0; i < count; i++) {
            if (u32(p) != 0x02014b50L) {
                throw new IOException("bad central directory");
            }
            int method = u16(p + 10);
            long dosTime = u32(p + 12);
            long csize = u32(p + 20);
            long size = u32(p + 24);
            int nlen = u16(p + 28);
            int elen = u16(p + 30);
            int clen = u16(p + 32);
            long lho = u32(p + 42);
            String name = new String(data, p + 46, nlen, StandardCharsets.UTF_8);
            p += 46 + nlen + elen + clen;
            int lnlen = u16((int) lho + 26);
            int lelen = u16((int) lho + 28);
            long dataOffset = lho + 30 + lnlen + lelen;
            boolean dir = name.endsWith("/");
            String path = "/" + (dir ? name.substring(0, name.length() - 1) : name);
            if (path.equals("/")) {
                continue;
            }
            addNode(path, new Node(path, dir, method, csize, size, dataOffset, dosToMillis(dosTime)));
        }
    }

    private void addNode(String path, Node node) {
        Node existing = nodes.get(path);
        if (existing != null && existing.directory && node.directory) {
            return;
        }
        nodes.put(path, node);
        String parent = parentOf(path);
        while (true) {
            Node p = nodes.get(parent);
            if (p == null) {
                p = new Node(parent, true, 0, 0, 0, 0, 0);
                nodes.put(parent, p);
            }
            p.children.add(path.substring(path.lastIndexOf('/') + 1));
            if (parent.equals("/")) {
                break;
            }
            path = parent;
            parent = parentOf(parent);
        }
    }

    private static String parentOf(String path) {
        int i = path.lastIndexOf('/');
        return i <= 0 ? "/" : path.substring(0, i);
    }

    private static long dosToMillis(long dos) {
        int date = (int) (dos >>> 16);
        int time = (int) (dos & 0xffff);
        int year = 1980 + ((date >> 9) & 0x7f);
        int month = Math.max(1, (date >> 5) & 0xf);
        int day = Math.max(1, date & 0x1f);
        int hour = (time >> 11) & 0x1f;
        int minute = (time >> 5) & 0x3f;
        int second = (time & 0x1f) * 2;
        return java.time.LocalDateTime.of(year, month, day, Math.min(hour, 23), Math.min(minute, 59),
                Math.min(second, 59)).toInstant(java.time.ZoneOffset.UTC).toEpochMilli();
    }

    public Node get(String path) {
        return nodes.get(path);
    }

    public List<String> childNames(String path) {
        Node n = nodes.get(path);
        return n == null ? new ArrayList<>() : new ArrayList<>(n.children);
    }

    /** Inflates an entry. */
    public byte[] read(Node node) throws IOException {
        if (node.directory) {
            throw new IOException("is a directory: " + node.name);
        }
        int offset = (int) node.dataOffset;
        if (node.method == 0) {
            byte[] out = new byte[(int) node.size];
            System.arraycopy(data, offset, out, 0, out.length);
            return out;
        }
        if (node.method != 8) {
            throw new IOException("unsupported zip method " + node.method);
        }
        Inflater inflater = new Inflater(true);
        try {
            inflater.setInput(data, offset, (int) node.compressedSize);
            byte[] out = new byte[(int) node.size];
            int done = 0;
            while (done < out.length) {
                int n = inflater.inflate(out, done, out.length - done);
                if (n == 0 && (inflater.finished() || inflater.needsInput())) {
                    break;
                }
                done += n;
            }
            return out;
        } catch (DataFormatException e) {
            throw new IOException(e);
        } finally {
            inflater.end();
        }
    }
}
