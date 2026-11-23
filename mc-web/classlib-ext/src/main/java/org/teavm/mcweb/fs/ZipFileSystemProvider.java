package org.teavm.mcweb.fs;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.URI;
import java.nio.channels.SeekableByteChannel;
import java.nio.file.AccessMode;
import java.nio.file.CopyOption;
import java.nio.file.DirectoryStream;
import java.nio.file.FileStore;
import java.nio.file.FileSystem;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.NoSuchFileException;
import java.nio.file.NotDirectoryException;
import java.nio.file.OpenOption;
import java.nio.file.Path;
import java.nio.file.ReadOnlyFileSystemException;
import java.nio.file.attribute.BasicFileAttributeView;
import java.nio.file.attribute.BasicFileAttributes;
import java.nio.file.attribute.FileAttribute;
import java.nio.file.attribute.FileAttributeView;
import java.nio.file.attribute.FileTime;
import java.nio.file.spi.FileSystemProvider;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * The "jar" provider the game looks up to open zip files (the game jar, resource packs). It reads the
 * archive bytes through the normal file system and serves entries from memory.
 */
public final class ZipFileSystemProvider extends FileSystemProvider {
    @Override
    public String getScheme() {
        return "jar";
    }

    @Override
    public FileSystem newFileSystem(URI uri, Map<String, ?> env) throws IOException {
        String spec = uri.getSchemeSpecificPart();
        int bang = spec.indexOf("!/");
        String file = bang >= 0 ? spec.substring(0, bang) : spec;
        return newFileSystem(java.nio.file.Paths.get(URI.create(file)), env);
    }

    @Override
    public FileSystem newFileSystem(Path path, Map<String, ?> env) throws IOException {
        return new ZipFileSystem(this, new ZipImage(Files.readAllBytes(path)));
    }

    /** For callers that already hold the archive bytes (the page loader). */
    public FileSystem newFileSystem(byte[] archive) throws IOException {
        return new ZipFileSystem(this, new ZipImage(archive));
    }

    @Override
    public FileSystem getFileSystem(URI uri) {
        throw new java.nio.file.FileSystemNotFoundException(uri.toString());
    }

    @Override
    public Path getPath(URI uri) {
        throw new java.nio.file.FileSystemNotFoundException(uri.toString());
    }

    private static ZipPath zip(Path p) {
        return (ZipPath) p.toAbsolutePath().normalize();
    }

    private static ZipImage.Node node(Path p) throws IOException {
        ZipPath zp = zip(p);
        ZipImage.Node n = ((ZipFileSystem) zp.getFileSystem()).image.get(zp.value());
        if (n == null) {
            throw new NoSuchFileException(p.toString());
        }
        return n;
    }

    @Override
    public InputStream newInputStream(Path path, OpenOption... options) throws IOException {
        ZipImage.Node n = node(path);
        return new ByteArrayInputStream(((ZipFileSystem) path.getFileSystem()).image.read(n));
    }

    @Override
    public SeekableByteChannel newByteChannel(Path path, Set<? extends OpenOption> options,
            FileAttribute<?>... attrs) throws IOException {
        throw new UnsupportedOperationException("byte channels over zip entries are not supported");
    }

    @Override
    public DirectoryStream<Path> newDirectoryStream(Path dir, DirectoryStream.Filter<? super Path> filter)
            throws IOException {
        ZipImage.Node n = node(dir);
        if (!n.isDirectory()) {
            throw new NotDirectoryException(dir.toString());
        }
        ZipPath zp = zip(dir);
        ZipFileSystem fs = (ZipFileSystem) zp.getFileSystem();
        List<Path> out = new ArrayList<>();
        for (String child : fs.image.childNames(zp.value())) {
            Path p = new ZipPath(fs, zp.value().equals("/") ? "/" + child : zp.value() + "/" + child);
            if (filter == null || filter.accept(p)) {
                out.add(p);
            }
        }
        return new DirectoryStream<Path>() {
            @Override
            public Iterator<Path> iterator() {
                return out.iterator();
            }

            @Override
            public void close() {
            }
        };
    }

    @Override
    public void createDirectory(Path dir, FileAttribute<?>... attrs) {
        throw new ReadOnlyFileSystemException();
    }

    @Override
    public void delete(Path path) {
        throw new ReadOnlyFileSystemException();
    }

    @Override
    public void copy(Path source, Path target, CopyOption... options) {
        throw new ReadOnlyFileSystemException();
    }

    @Override
    public void move(Path source, Path target, CopyOption... options) {
        throw new ReadOnlyFileSystemException();
    }

    @Override
    public boolean isSameFile(Path path, Path path2) {
        return path.equals(path2);
    }

    @Override
    public boolean isHidden(Path path) {
        return false;
    }

    @Override
    public FileStore getFileStore(Path path) {
        throw new UnsupportedOperationException();
    }

    @Override
    public void checkAccess(Path path, AccessMode... modes) throws IOException {
        node(path);
        for (AccessMode m : modes) {
            if (m == AccessMode.WRITE) {
                throw new java.nio.file.AccessDeniedException(path.toString());
            }
        }
    }

    @Override
    public <V extends FileAttributeView> V getFileAttributeView(Path path, Class<V> type, LinkOption... options) {
        return null;
    }

    @Override
    @SuppressWarnings("unchecked")
    public <A extends BasicFileAttributes> A readAttributes(Path path, Class<A> type, LinkOption... options)
            throws IOException {
        ZipImage.Node n = node(path);
        return (A) new BasicFileAttributes() {
            @Override
            public FileTime lastModifiedTime() {
                return FileTime.fromMillis(n.lastModified());
            }

            @Override
            public FileTime lastAccessTime() {
                return lastModifiedTime();
            }

            @Override
            public FileTime creationTime() {
                return lastModifiedTime();
            }

            @Override
            public boolean isRegularFile() {
                return !n.isDirectory();
            }

            @Override
            public boolean isDirectory() {
                return n.isDirectory();
            }

            @Override
            public boolean isSymbolicLink() {
                return false;
            }

            @Override
            public boolean isOther() {
                return false;
            }

            @Override
            public long size() {
                return n.size();
            }

            @Override
            public Object fileKey() {
                return null;
            }
        };
    }

    @Override
    public Map<String, Object> readAttributes(Path path, String attributes, LinkOption... options)
            throws IOException {
        throw new UnsupportedOperationException();
    }

    @Override
    public void setAttribute(Path path, String attribute, Object value, LinkOption... options) {
        throw new ReadOnlyFileSystemException();
    }
}
