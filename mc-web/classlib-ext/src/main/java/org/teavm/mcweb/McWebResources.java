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
    }

    /**
     * Unpacks the archive into the (in-memory) default file system. The game locates its assets by
     * asking for a marker resource and treating the parent directory of the returned {@code file:} URL
     * as a normal directory, so the files have to exist there.
     */
    private static void mount() throws IOException {
        mountDirectory("/");
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
