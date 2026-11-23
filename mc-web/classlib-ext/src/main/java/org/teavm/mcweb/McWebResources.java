package org.teavm.mcweb;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import org.teavm.mcweb.fs.ZipImage;

/**
 * Classpath resources for the browser build. The page supplies one zip archive holding the game's
 * assets and data; {@code getResourceAsStream} calls are redirected here and read from it.
 */
public final class McWebResources {
    private static ZipImage image;

    private McWebResources() {
    }

    public static void install(byte[] archive) throws IOException {
        image = new ZipImage(archive);
        mount();
        selfCheck("/assets/minecraft/textures/gui/title/mojangstudios.png");
    }

    /**
     * Unpacks the archive into the (in-memory) default file system. The game locates its assets by
     * asking for a marker resource and treating the parent directory of the returned {@code file:} URL
     * as a normal directory, so the files have to exist there.
     */
    private static void mount() throws IOException {
        mountDirectory("/");
    }

    private static void selfCheck(String path) throws IOException {
        ZipImage.Node node = image.get(path);
        byte[] expected = image.read(node);
        java.io.File f = new java.io.File(path);
        java.io.InputStream in = new java.io.FileInputStream(f);
        byte[] actual = in.readAllBytes();
        in.close();
        byte[] viaFiles = java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path));
        org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] zip=" + expected.length + " first=" + (expected[0] & 0xff)
                + " fileInputStream=" + actual.length + " first=" + (actual.length > 0 ? actual[0] & 0xff : -1)
                + " Files.readAllBytes=" + viaFiles.length + " first=" + (viaFiles.length > 0 ? viaFiles[0] & 0xff : -1)
                + " fileLength=" + f.length());
        java.nio.ByteBuffer wrapped = java.nio.ByteBuffer.wrap(expected);
        wrapped.order(java.nio.ByteOrder.BIG_ENDIAN);
        long viaWrap = wrapped.getLong(0);
        java.nio.file.Path base = java.nio.file.Paths.get("/assets/minecraft");
        java.nio.file.Path child = java.nio.file.Paths.get("/assets/minecraft/textures/x.png");
        java.nio.file.Path rel = base.relativize(child);
        StringBuilder names = new StringBuilder();
        if (rel != null) {
            for (java.nio.file.Path part : rel) {
                names.append('[').append(part).append(']');
            }
        }
        org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] relativize=" + rel + " iter=" + names);
        try {
            com.google.common.base.Joiner joiner = com.google.common.base.Joiner.on("/");
            org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] joiner=" + joiner.join(java.util.List.of("a", "b")));
            org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] joiner path=" + joiner.join(rel));
            java.nio.file.Path empty = base.relativize(base);
            org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] joiner empty=" + joiner.join(empty));
        } catch (Throwable t) {
            org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] joiner failed: " + t);
        }
        java.nio.ByteBuffer probe = java.nio.ByteBuffer.wrap(expected.clone());
        probe.getLong(0);
        int afterLong = probe.position();
        probe.getInt(0);
        int afterInt = probe.position();
        probe.get(0);
        probe.getShort(0);
        org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] positions after absolute getLong/getInt/get/getShort: "
                + afterLong + " " + afterInt + " " + probe.position());
        java.nio.ByteBuffer le = java.nio.ByteBuffer.wrap(expected.clone());
        le.order(java.nio.ByteOrder.LITTLE_ENDIAN);
        le.order(java.nio.ByteOrder.BIG_ENDIAN);
        org.teavm.mcweb.host.WebHost.consoleLog("[selfcheck] getLong wrap=" + Long.toHexString(viaWrap)
                + " flipped=" + Long.toHexString(le.getLong(0)) + " expected=89504e470d0a1a0a eq=" + (viaWrap == 0x89504E470D0A1A0AL)
                + " int=" + Integer.toHexString(wrapped.getInt(0)) + " int8=" + Integer.toHexString(wrapped.getInt(8)));
    }

    /** Like {@code Files.createDirectories}, which TeaVM's version fails on existing directories. */
    public static void ensureDirectory(java.nio.file.Path dir) throws IOException {
        if (dir == null || java.nio.file.Files.isDirectory(dir)) {
            return;
        }
        ensureDirectory(dir.getParent());
        try {
            java.nio.file.Files.createDirectory(dir);
        } catch (java.nio.file.FileAlreadyExistsException e) {
            // created concurrently or by an earlier call
        }
    }

    private static void mountDirectory(String dir) throws IOException {
        for (String name : image.childNames(dir)) {
            String path = dir.equals("/") ? "/" + name : dir + "/" + name;
            ZipImage.Node node = image.get(path);
            if (node.isDirectory()) {
                ensureDirectory(java.nio.file.Paths.get(path));
                mountDirectory(path);
            } else {
                java.nio.file.Path target = java.nio.file.Paths.get(path);
                ensureDirectory(target.getParent());
                java.nio.file.Files.write(target, image.read(node));
            }
        }
    }

    /** A {@code file:} URL for an archive entry (which was mounted into the default file system), or null. */
    public static java.net.URL url(String name) {
        if (image == null || name == null) {
            return null;
        }
        String path = clean(name);
        if (image.get(path) == null) {
            return null;
        }
        try {
            return new java.net.URL("file://" + path);
        } catch (java.net.MalformedURLException e) {
            return null;
        }
    }

    public static boolean isInstalled() {
        return image != null;
    }

    public static ZipImage image() {
        return image;
    }

    private static String clean(String name) {
        String n = name;
        while (n.startsWith("/")) {
            n = n.substring(1);
        }
        return "/" + n;
    }

    /** Opens an archive entry by absolute path (leading slash optional); null when missing. */
    public static InputStream open(String name) {
        if (image == null || name == null) {
            return null;
        }
        ZipImage.Node node = image.get(clean(name));
        if (node == null || node.isDirectory()) {
            return null;
        }
        try {
            return new ByteArrayInputStream(image.read(node));
        } catch (IOException e) {
            return null;
        }
    }

    /** {@code Class.getResourceAsStream}: relative names resolve against the class's package. */
    public static InputStream open(Class<?> base, String name) {
        if (name == null) {
            throw new NullPointerException();
        }
        if (name.startsWith("/")) {
            return open(name);
        }
        String cn = base.getName();
        int dot = cn.lastIndexOf('.');
        String pkg = dot < 0 ? "" : cn.substring(0, dot).replace('.', '/') + "/";
        return open(pkg + name);
    }
}
