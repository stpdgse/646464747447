package org.teavm.mcweb.fs;

import java.io.IOException;
import java.net.URI;
import java.nio.file.FileSystem;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.nio.file.WatchEvent;
import java.nio.file.WatchKey;
import java.nio.file.WatchService;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;

/** Slash-separated path inside a {@link ZipFileSystem}. */
final class ZipPath implements Path {
    private final ZipFileSystem fs;
    private final String path;
    private String[] names;

    ZipPath(ZipFileSystem fs, String path) {
        this.fs = fs;
        this.path = clean(path);
    }

    private static String clean(String p) {
        StringBuilder sb = new StringBuilder();
        boolean slash = false;
        for (int i = 0; i < p.length(); i++) {
            char c = p.charAt(i);
            if (c == '/') {
                if (!slash) {
                    sb.append(c);
                }
                slash = true;
            } else {
                sb.append(c);
                slash = false;
            }
        }
        if (sb.length() > 1 && sb.charAt(sb.length() - 1) == '/') {
            sb.setLength(sb.length() - 1);
        }
        return sb.toString();
    }

    String value() {
        return path;
    }

    private String[] names() {
        if (names == null) {
            List<String> out = new ArrayList<>();
            for (String s : path.split("/")) {
                if (!s.isEmpty()) {
                    out.add(s);
                }
            }
            names = out.toArray(new String[0]);
        }
        return names;
    }

    @Override
    public FileSystem getFileSystem() {
        return fs;
    }

    @Override
    public boolean isAbsolute() {
        return path.startsWith("/");
    }

    @Override
    public Path getRoot() {
        return isAbsolute() ? new ZipPath(fs, "/") : null;
    }

    @Override
    public Path getFileName() {
        String[] n = names();
        return n.length == 0 ? null : new ZipPath(fs, n[n.length - 1]);
    }

    @Override
    public Path getParent() {
        String[] n = names();
        if (n.length == 0) {
            return null;
        }
        if (n.length == 1) {
            return isAbsolute() ? new ZipPath(fs, "/") : null;
        }
        return new ZipPath(fs, (isAbsolute() ? "/" : "") + String.join("/", java.util.Arrays.copyOf(n, n.length - 1)));
    }

    @Override
    public int getNameCount() {
        return names().length;
    }

    @Override
    public Path getName(int index) {
        return new ZipPath(fs, names()[index]);
    }

    @Override
    public Path subpath(int beginIndex, int endIndex) {
        String[] n = names();
        if (beginIndex < 0 || endIndex > n.length || beginIndex >= endIndex) {
            throw new IllegalArgumentException();
        }
        return new ZipPath(fs, String.join("/", java.util.Arrays.copyOfRange(n, beginIndex, endIndex)));
    }

    @Override
    public boolean startsWith(Path other) {
        if (!(other instanceof ZipPath)) {
            return false;
        }
        ZipPath o = (ZipPath) other;
        if (isAbsolute() != o.isAbsolute()) {
            return false;
        }
        String[] a = names();
        String[] b = o.names();
        if (b.length > a.length) {
            return false;
        }
        for (int i = 0; i < b.length; i++) {
            if (!a[i].equals(b[i])) {
                return false;
            }
        }
        return true;
    }

    @Override
    public boolean endsWith(Path other) {
        if (!(other instanceof ZipPath)) {
            return false;
        }
        ZipPath o = (ZipPath) other;
        String[] a = names();
        String[] b = o.names();
        if (o.isAbsolute()) {
            return isAbsolute() && java.util.Arrays.equals(a, b);
        }
        if (b.length > a.length) {
            return false;
        }
        for (int i = 0; i < b.length; i++) {
            if (!a[a.length - b.length + i].equals(b[i])) {
                return false;
            }
        }
        return true;
    }

    @Override
    public Path normalize() {
        List<String> out = new ArrayList<>();
        for (String s : names()) {
            if (s.equals(".")) {
                continue;
            }
            if (s.equals("..")) {
                if (!out.isEmpty() && !out.get(out.size() - 1).equals("..")) {
                    out.remove(out.size() - 1);
                    continue;
                }
                if (isAbsolute()) {
                    continue;
                }
            }
            out.add(s);
        }
        return new ZipPath(fs, (isAbsolute() ? "/" : "") + String.join("/", out));
    }

    @Override
    public Path resolve(Path other) {
        if (!(other instanceof ZipPath)) {
            throw new java.nio.file.ProviderMismatchException();
        }
        ZipPath o = (ZipPath) other;
        if (o.isAbsolute()) {
            return o;
        }
        if (o.path.isEmpty()) {
            return this;
        }
        if (path.isEmpty()) {
            return o;
        }
        return new ZipPath(fs, path + "/" + o.path);
    }

    @Override
    public Path relativize(Path other) {
        ZipPath o = (ZipPath) other;
        if (isAbsolute() != o.isAbsolute()) {
            throw new IllegalArgumentException("cannot relativize");
        }
        String[] a = names();
        String[] b = o.names();
        int common = 0;
        while (common < a.length && common < b.length && a[common].equals(b[common])) {
            common++;
        }
        List<String> out = new ArrayList<>();
        for (int i = common; i < a.length; i++) {
            out.add("..");
        }
        for (int i = common; i < b.length; i++) {
            out.add(b[i]);
        }
        return new ZipPath(fs, String.join("/", out));
    }

    @Override
    public URI toUri() {
        return URI.create("jar:file:///zip!" + (isAbsolute() ? path : "/" + path));
    }

    @Override
    public Path toAbsolutePath() {
        return isAbsolute() ? this : new ZipPath(fs, "/" + path);
    }

    @Override
    public Path toRealPath(LinkOption... options) throws IOException {
        return toAbsolutePath().normalize();
    }

    @Override
    public WatchKey register(WatchService watcher, WatchEvent.Kind<?>[] events, WatchEvent.Modifier... modifiers) {
        throw new UnsupportedOperationException();
    }

    @Override
    public int compareTo(Path other) {
        return path.compareTo(((ZipPath) other).path);
    }

    @Override
    public boolean equals(Object o) {
        return o instanceof ZipPath && ((ZipPath) o).fs == fs && ((ZipPath) o).path.equals(path);
    }

    @Override
    public int hashCode() {
        return path.hashCode();
    }

    @Override
    public String toString() {
        return path;
    }

    @Override
    public Iterator<Path> iterator() {
        List<Path> out = new ArrayList<>();
        for (String s : names()) {
            out.add(new ZipPath(fs, s));
        }
        return out.iterator();
    }
}
