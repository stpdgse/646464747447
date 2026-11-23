package org.teavm.mcweb;

import java.util.Collections;
import java.util.Date;
import java.util.Enumeration;
import java.util.Locale;
import java.util.Map;
import java.util.SortedMap;
import java.util.SortedSet;
import java.util.TreeMap;
import java.util.TreeSet;
import java.util.function.Predicate;
import java.util.regex.Pattern;

/**
 * Real implementations for JDK methods the browser class library lacks. {@link JdkGapTransformer}
 * redirects calls here by name and signature: for an instance method the receiver is the first
 * parameter. Anything not listed here becomes a generated "not available" stub.
 */
public final class JdkShims {
    private JdkShims() {
    }

    // classpath resources ------------------------------------------------------------------------

    /** Minecraft.fillSystemReport: leave the report as is (the window and GPU may not exist yet). */
    public static net.minecraft.SystemReport keepReport(net.minecraft.SystemReport report,
            net.minecraft.client.Minecraft minecraft, net.minecraft.client.resources.language.LanguageManager language,
            String launchedVersion, net.minecraft.client.Options options) {
        return report;
    }

    /** Paths.get(URI): TeaVM rejects file URIs without an authority; use the path component. */
    public static java.nio.file.Path get(java.net.URI uri) {
        if ("file".equals(uri.getScheme())) {
            return java.nio.file.Paths.get(uri.getPath());
        }
        throw new IllegalArgumentException("unsupported URI scheme: " + uri);
    }

    public static java.nio.file.Path of(java.net.URI uri) {
        return get(uri);
    }

    /** Files.createDirectories that tolerates existing directories (TeaVM's throws). */
    public static java.nio.file.Path createDirectories(java.nio.file.Path dir,
            java.nio.file.attribute.FileAttribute<?>[] attrs) throws java.io.IOException {
        McWebResources.ensureDirectory(dir);
        return dir;
    }

    public static java.io.InputStream getResourceAsStream(Class<?> c, String name) {
        return McWebResources.open(c, name);
    }

    public static java.io.InputStream getResourceAsStream(ClassLoader cl, String name) {
        return McWebResources.open(name);
    }

    public static java.io.InputStream getSystemResourceAsStream(String name) {
        return McWebResources.open(name);
    }

    // game replacements ---------------------------------------------------------------------------

    /** DataFixers.optimize: world-upgrade optimisation is skipped (new worlds need no data fixing). */
    public static java.util.concurrent.CompletableFuture<?> noOptimize(java.util.Set<?> types) {
        return java.util.concurrent.CompletableFuture.completedFuture(null);
    }

    /** Util.doFetchChoiceType: entity/block-entity data-fixer type lookup, only used for upgrading old data. */
    public static com.mojang.datafixers.types.Type<?> nullChoiceType(com.mojang.datafixers.DSL.TypeReference type,
            String id) {
        return null;
    }

    /** DataFixers.getDataFixer: a pass-through fixer (see NoopDataFixer). */
    public static com.mojang.datafixers.DataFixer noDataFixer() {
        return org.teavm.mcweb.game.NoopDataFixer.INSTANCE;
    }

    public static void gapHit(String message) {
        org.teavm.mcweb.host.WebHost.logStack("[gap] " + message);
    }


    // generic supertypes ------------------------------------------------------------------------

    /** Minimal ParameterizedType for the few anonymous TypeToken subclasses the game uses. */
    private static final class Param implements java.lang.reflect.ParameterizedType {
        private final Class<?> raw;
        private final java.lang.reflect.Type[] args;

        Param(Class<?> raw, java.lang.reflect.Type... args) {
            this.raw = raw;
            this.args = args;
        }

        @Override
        public java.lang.reflect.Type[] getActualTypeArguments() {
            return args.clone();
        }

        @Override
        public java.lang.reflect.Type getRawType() {
            return raw;
        }

        @Override
        public java.lang.reflect.Type getOwnerType() {
            return null;
        }

        @Override
        public boolean equals(Object o) {
            return o instanceof java.lang.reflect.ParameterizedType p && p.getRawType() == raw
                    && java.util.Arrays.equals(p.getActualTypeArguments(), args);
        }

        @Override
        public int hashCode() {
            return java.util.Arrays.hashCode(args) ^ raw.hashCode();
        }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder(raw.getName()).append('<');
            for (int i = 0; i < args.length; i++) {
                sb.append(i > 0 ? ", " : "").append(args[i] instanceof Class<?> c ? c.getName() : args[i].toString());
            }
            return sb.append('>').toString();
        }
    }

    /** Class.getGenericSuperclass: TeaVM keeps no generic class signatures, so known cases are tabulated. */
    public static java.lang.reflect.Type getGenericSuperclass(Class<?> c) {
        String n = c.getName();
        if (n.equals("net.minecraft.client.Options$1")) {
            return new Param(com.google.gson.reflect.TypeToken.class, new Param(java.util.List.class, String.class));
        }
        if (n.equals("net.minecraft.client.sounds.SoundManager$1")) {
            return new Param(com.google.gson.reflect.TypeToken.class,
                    new Param(java.util.Map.class, String.class,
                            net.minecraft.client.resources.sounds.SoundEventRegistration.class));
        }
        return c.getSuperclass();
    }

    public static boolean isAnonymousClass(Class<?> c) {
        String n = c.getName();
        int i = n.lastIndexOf('$');
        if (i < 0 || i == n.length() - 1) {
            return false;
        }
        for (int j = i + 1; j < n.length(); j++) {
            if (!Character.isDigit(n.charAt(j))) {
                return false;
            }
        }
        return true;
    }

    public static java.lang.reflect.Type[] getGenericInterfaces(Class<?> c) {
        return c.getInterfaces();
    }

    // wait / notify without monitors ------------------------------------------------------------

    private static int waitLogged;

    private static void logWait(String what, Object o) {
        if (waitLogged++ < 3) {
            org.teavm.mcweb.host.WebHost.logStack("[wait] " + what + " on " + o.getClass().getName());
        }
    }

    public static void wait(Object o) throws InterruptedException {
        logWait("wait()", o);
        Thread.sleep(1);
    }

    public static void wait(Object o, long timeoutMillis) throws InterruptedException {
        logWait("wait(" + timeoutMillis + ")", o);
        Thread.sleep(1);
    }

    public static void wait(Object o, long timeoutMillis, int nanos) throws InterruptedException {
        Thread.sleep(1);
    }

    public static void notify(Object o) {
    }

    public static void notifyAll(Object o) {
    }

    // debugging --------------------------------------------------------------------------------

    public static void trace(String message) {
        org.teavm.mcweb.host.WebHost.logStack("[trace] " + message);
    }

    public static void traceValue(String label, Object value) {
        System.out.println("[trace] " + label + " = " + value);
    }

    /** Files.find: TeaVM's version dereferences a null stream when the start path itself does not match. */
    public static java.util.stream.Stream<java.nio.file.Path> find(java.nio.file.Path start, int maxDepth,
            java.util.function.BiPredicate<java.nio.file.Path, java.nio.file.attribute.BasicFileAttributes> matcher,
            java.nio.file.FileVisitOption[] options) throws java.io.IOException {
        java.util.List<java.nio.file.Path> found = new java.util.ArrayList<>();
        findInto(start, maxDepth, matcher, found);
        return found.stream();
    }

    private static void findInto(java.nio.file.Path path, int depth,
            java.util.function.BiPredicate<java.nio.file.Path, java.nio.file.attribute.BasicFileAttributes> matcher,
            java.util.List<java.nio.file.Path> out) throws java.io.IOException {
        java.nio.file.attribute.BasicFileAttributes attributes =
                java.nio.file.Files.readAttributes(path, java.nio.file.attribute.BasicFileAttributes.class);
        if (matcher.test(path, attributes)) {
            out.add(path);
        }
        if (depth > 0 && attributes.isDirectory()) {
            try (java.nio.file.DirectoryStream<java.nio.file.Path> children = java.nio.file.Files.newDirectoryStream(path)) {
                for (java.nio.file.Path child : children) {
                    findInto(child, depth - 1, matcher, out);
                }
            }
        }
    }

    public static java.nio.channels.SeekableByteChannel newByteChannel(java.nio.file.Path path,
            java.nio.file.OpenOption[] options) throws java.io.IOException {
        return newByteChannel(path, new java.util.HashSet<>(java.util.Arrays.asList(options)),
                new java.nio.file.attribute.FileAttribute<?>[0]);
    }

    public static java.nio.channels.SeekableByteChannel newByteChannel(java.nio.file.Path path,
            java.util.Set<? extends java.nio.file.OpenOption> options,
            java.nio.file.attribute.FileAttribute<?>[] attributes) throws java.io.IOException {
        java.nio.file.spi.FileSystemProvider provider = path.getFileSystem().provider();
        if (provider.getClass().getName().endsWith("DefaultFileSystemProvider")) {
            return java.nio.channels.FileChannel.open(path, options);
        }
        return provider.newByteChannel(path, options, attributes);
    }

    /** Files.newInputStream: TeaVM's default provider does not implement it; paths of the default file system use java.io. */
    public static java.io.InputStream newInputStream(java.nio.file.Path path, java.nio.file.OpenOption[] options)
            throws java.io.IOException {
        java.nio.file.spi.FileSystemProvider provider = path.getFileSystem().provider();
        if (provider instanceof org.teavm.mcweb.fs.ZipFileSystemProvider) {
            return provider.newInputStream(path, options);
        }
        if (!provider.getClass().getName().endsWith("DefaultFileSystemProvider")) {
            // a game-supplied file system (like the pack link file system)
            return provider.newInputStream(path, options);
        }
        java.io.File file = new java.io.File(path.toString());
        if (!file.exists()) {
            throw new java.nio.file.NoSuchFileException(path.toString());
        }
        if (file.isDirectory()) {
            throw new java.io.IOException(path + ": is a directory");
        }
        return new java.io.FileInputStream(file);
    }

    public static java.io.OutputStream newOutputStream(java.nio.file.Path path, java.nio.file.OpenOption[] options)
            throws java.io.IOException {
        java.nio.file.spi.FileSystemProvider provider = path.getFileSystem().provider();
        if (provider instanceof org.teavm.mcweb.fs.ZipFileSystemProvider) {
            return provider.newOutputStream(path, options);
        }
        java.util.Set<java.nio.file.OpenOption> set = new java.util.HashSet<>(java.util.Arrays.asList(options));
        java.io.File file = new java.io.File(path.toString());
        boolean create = set.isEmpty() || set.contains(java.nio.file.StandardOpenOption.CREATE)
                || set.contains(java.nio.file.StandardOpenOption.CREATE_NEW);
        if (set.contains(java.nio.file.StandardOpenOption.CREATE_NEW) && file.exists()) {
            throw new java.nio.file.FileAlreadyExistsException(path.toString());
        }
        if (!file.exists() && !create) {
            throw new java.nio.file.NoSuchFileException(path.toString());
        }
        java.io.File parent = file.getParentFile();
        if (parent != null && !parent.exists()) {
            throw new java.nio.file.NoSuchFileException(path.toString());
        }
        return new java.io.FileOutputStream(file, set.contains(java.nio.file.StandardOpenOption.APPEND));
    }

    /**
     * PathPackResources.listPath without Guava's Joiner (which hits a null dereference on paths in this build):
     * lists every regular file below the resolved directory and reports it as an identifier.
     */
    public static void listPath(String namespace, java.nio.file.Path root, java.util.List<String> directories,
            net.minecraft.server.packs.PackResources.ResourceOutput output) {
        java.nio.file.Path directory = net.minecraft.util.FileUtil.resolvePath(root, directories);
        java.util.List<java.nio.file.Path> files;
        try {
            files = new java.util.ArrayList<>();
            findInto(directory, Integer.MAX_VALUE, (p, a) -> a.isRegularFile(), files);
        } catch (java.nio.file.NoSuchFileException e) {
            return;
        } catch (java.io.IOException e) {
            org.teavm.mcweb.host.WebHost.consoleWarn("Failed to list path " + directory + ": " + e);
            return;
        }
        org.teavm.mcweb.host.WebHost.consoleLog("[listPath] ns=" + namespace + " root=" + root + " (" + root.getClass().getName()
                + ") dir=" + directory + " n=" + files.size() + (files.isEmpty() ? "" : " first=" + files.get(0)
                + " (" + files.get(0).getClass().getName() + ")"));
        for (java.nio.file.Path path : files) {
            StringBuilder name = new StringBuilder();
            for (java.nio.file.Path part : root.relativize(path)) {
                if (name.length() > 0) {
                    name.append('/');
                }
                name.append(part.toString());
            }
            net.minecraft.resources.Identifier id = net.minecraft.resources.Identifier.tryBuild(namespace, name.toString());
            if (id == null) {
                org.teavm.mcweb.host.WebHost.consoleWarn("Invalid path in pack: " + namespace + ":" + name + ", ignoring");
            } else {
                output.accept(id, net.minecraft.server.packs.resources.IoSupplier.create(path));
            }
        }
    }

    // java.io / java.nio.file --------------------------------------------------------------------

    public static java.nio.file.Path toPath(java.io.File file) {
        return java.nio.file.Paths.get(file.getPath());
    }

    /** Reader.readAllAsString (JDK 26): drains the reader into one string. */
    public static String readAllAsString(java.io.Reader in) throws java.io.IOException {
        StringBuilder sb = new StringBuilder();
        char[] buf = new char[4096];
        int n;
        while ((n = in.read(buf, 0, buf.length)) >= 0) {
            sb.append(buf, 0, n);
        }
        return sb.toString();
    }

    public static java.util.List<String> readAllLines(java.io.BufferedReader in) throws java.io.IOException {
        java.util.List<String> lines = new java.util.ArrayList<>();
        String line;
        while ((line = in.readLine()) != null) {
            lines.add(line);
        }
        return lines;
    }

    // java.io (continued) ----------------------------------------------------------------------------------

    public static long transferTo(java.io.BufferedReader in, java.io.Writer out) throws java.io.IOException {
        return transferTo((java.io.Reader) in, out);
    }

    public static long transferTo(java.io.Reader in, java.io.Writer out) throws java.io.IOException {
        char[] buf = new char[4096];
        long total = 0;
        int n;
        while ((n = in.read(buf, 0, buf.length)) >= 0) {
            out.write(buf, 0, n);
            total += n;
        }
        return total;
    }

    // java.lang.System / Runtime ---------------------------------------------------------------

    public static Map<String, String> getenv() {
        return Collections.emptyMap();
    }

    public static void exit(int status) {
        throw new IllegalStateException("System.exit(" + status + ") called");
    }

    public static void load(String path) {
        throw new UnsatisfiedLinkError("native libraries are not available in the browser: " + path);
    }

    public static void loadLibrary(String name) {
        throw new UnsatisfiedLinkError("native libraries are not available in the browser: " + name);
    }

    public static String mapLibraryName(String name) {
        return name;
    }

    public static long maxMemory(Runtime rt) {
        return 2L * 1024 * 1024 * 1024;
    }

    public static void addShutdownHook(Runtime rt, Thread hook) {
    }

    public static boolean removeShutdownHook(Runtime rt, Thread hook) {
        return false;
    }

    // java.lang.Thread / Math / Character -------------------------------------------------------

    /** Threads are cooperative fibers: a spin-wait must hand control back or nothing else can run. */
    public static void onSpinWait() {
        try {
            Thread.sleep(1);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
    }

    public static long threadId(Thread t) {
        return t.getId();
    }

    public static void dumpStack() {
        new Exception("Stack trace").printStackTrace();
    }

    public static float fma(float a, float b, float c) {
        return (float) ((double) a * (double) b + (double) c);
    }

    public static int powExact(int base, int exp) {
        long r = 1;
        for (int i = 0; i < exp; i++) {
            r *= base;
            if (r > Integer.MAX_VALUE || r < Integer.MIN_VALUE) {
                throw new ArithmeticException("integer overflow");
            }
        }
        return (int) r;
    }

    public static String toString(int codePoint) {
        return new String(Character.toChars(codePoint));
    }

    public static int parseUnsignedInt(String s, int radix) {
        long v = Long.parseLong(s, radix);
        if (v < 0 || v > 0xFFFFFFFFL) {
            throw new NumberFormatException("Illegal value for unsigned int: " + s);
        }
        return (int) v;
    }

    public static long parseUnsignedLong(String s, int radix) {
        if (s.startsWith("-")) {
            throw new NumberFormatException("Illegal leading minus sign on unsigned string " + s);
        }
        return new java.math.BigInteger(s, radix).longValue();
    }

    // java.lang.Class / ClassLoader / Package --------------------------------------------------

    public static java.net.URL getResource(ClassLoader cl, String name) {
        return McWebResources.url(name);
    }

    public static java.net.URL getSystemResource(String name) {
        return McWebResources.url(name);
    }

    public static Enumeration<java.net.URL> getResources(ClassLoader cl, String name) {
        java.net.URL u = McWebResources.url(name);
        return u == null ? Collections.<java.net.URL>emptyEnumeration() : Collections.enumeration(Collections.singletonList(u));
    }

    public static Enumeration<java.net.URL> getSystemResources(String name) {
        java.net.URL u = McWebResources.url(name);
        return u == null ? Collections.<java.net.URL>emptyEnumeration() : Collections.enumeration(Collections.singletonList(u));
    }

    public static java.net.URL getResource(Class<?> c, String name) {
        if (name != null && !name.startsWith("/")) {
            String cn = c.getName();
            int dot = cn.lastIndexOf('.');
            name = (dot < 0 ? "" : cn.substring(0, dot).replace('.', '/') + "/") + name;
        }
        return McWebResources.url(name);
    }

    public static Class<?> loadClass(ClassLoader cl, String name) throws ClassNotFoundException {
        return Class.forName(name);
    }

    public static String getSpecificationTitle(Package p) {
        return null;
    }

    public static String getSpecificationVersion(Package p) {
        return null;
    }

    public static Object[] getSigners(Class<?> c) {
        return null;
    }

    public static void setContextClassLoader(Thread t, ClassLoader cl) {
    }

    public static ThreadGroup getThreadGroup(Thread t) {
        return null;
    }

    public static long convert(java.util.concurrent.TimeUnit self, java.time.Duration duration) {
        long nanos;
        try {
            nanos = duration.toNanos();
        } catch (ArithmeticException e) {
            nanos = duration.isNegative() ? Long.MIN_VALUE : Long.MAX_VALUE;
        }
        return convert(self, nanos, java.util.concurrent.TimeUnit.NANOSECONDS);
    }

    public static long convert(java.util.concurrent.TimeUnit self, long duration,
            java.util.concurrent.TimeUnit source) {
        long nanos = source.toNanos(duration);
        switch (self) {
            case NANOSECONDS:
                return nanos;
            case MICROSECONDS:
                return nanos / 1000L;
            case MILLISECONDS:
                return nanos / 1_000_000L;
            case SECONDS:
                return nanos / 1_000_000_000L;
            case MINUTES:
                return nanos / 60_000_000_000L;
            case HOURS:
                return nanos / 3_600_000_000_000L;
            default:
                return nanos / 86_400_000_000_000L;
        }
    }

    // atomics (single-threaded cooperative scheduling: plain read-modify-write is atomic) -------------------

    public static Object compareAndExchange(java.util.concurrent.atomic.AtomicReferenceArray<Object> array, int index,
            Object expected, Object value) {
        Object witness = array.get(index);
        if (witness == expected) {
            array.set(index, value);
        }
        return witness;
    }

    public static Object compareAndExchange(java.util.concurrent.atomic.AtomicReference<Object> ref, Object expected,
            Object value) {
        Object witness = ref.get();
        if (witness == expected) {
            ref.set(value);
        }
        return witness;
    }

    public static int compareAndExchange(java.util.concurrent.atomic.AtomicInteger ref, int expected, int value) {
        int witness = ref.get();
        if (witness == expected) {
            ref.set(value);
        }
        return witness;
    }

    public static long compareAndExchange(java.util.concurrent.atomic.AtomicLong ref, long expected, long value) {
        long witness = ref.get();
        if (witness == expected) {
            ref.set(value);
        }
        return witness;
    }

    // java.util.stream ---------------------------------------------------------------------------

    private static final class IntSink implements java.util.function.IntConsumer {
        int[] data = new int[16];
        int size;

        @Override
        public void accept(int v) {
            if (size == data.length) {
                data = java.util.Arrays.copyOf(data, size * 2);
            }
            data[size++] = v;
        }
    }

    private static final class LongSink implements java.util.function.LongConsumer {
        long[] data = new long[16];
        int size;

        @Override
        public void accept(long v) {
            if (size == data.length) {
                data = java.util.Arrays.copyOf(data, size * 2);
            }
            data[size++] = v;
        }
    }

    /** Eager conversion: TeaVM has no spliterator-backed primitive streams. */
    public static java.util.stream.IntStream intStream(java.util.Spliterator.OfInt spliterator, boolean parallel) {
        IntSink sink = new IntSink();
        while (spliterator.tryAdvance((java.util.function.IntConsumer) sink)) {
            // drain
        }
        return java.util.Arrays.stream(sink.data, 0, sink.size);
    }

    public static java.util.stream.LongStream longStream(java.util.Spliterator.OfLong spliterator, boolean parallel) {
        LongSink sink = new LongSink();
        while (spliterator.tryAdvance((java.util.function.LongConsumer) sink)) {
            // drain
        }
        return java.util.Arrays.stream(sink.data, 0, sink.size);
    }

    // java.util --------------------------------------------------------------------------------

    public static Predicate<String> asPredicate(Pattern p) {
        return s -> p.matcher(s).find();
    }

    public static Date from(java.time.Instant instant) {
        return new Date(instant.toEpochMilli());
    }

    public static java.time.Instant toInstant(Date d) {
        return java.time.Instant.ofEpochMilli(d.getTime());
    }

    public static SortedMap<Object, Object> unmodifiableSortedMap(SortedMap<Object, Object> m) {
        return new TreeMap<>(m);
    }

    public static SortedSet<Object> unmodifiableSortedSet(SortedSet<Object> s) {
        return new TreeSet<>(s);
    }

    public static String getScript(Locale l) {
        return "";
    }

    public static String getISO3Country(Locale l) {
        String c = l.getCountry();
        switch (c) {
            case "":
                return "";
            case "US":
                return "USA";
            case "GB":
                return "GBR";
            case "DE":
                return "DEU";
            case "FR":
                return "FRA";
            case "ES":
                return "ESP";
            case "IT":
                return "ITA";
            case "JP":
                return "JPN";
            case "CN":
                return "CHN";
            case "BR":
                return "BRA";
            case "RU":
                return "RUS";
            default:
                return c;
        }
    }

    public static String getISO3Language(Locale l) {
        String c = l.getLanguage();
        switch (c) {
            case "en":
                return "eng";
            case "de":
                return "deu";
            case "fr":
                return "fra";
            case "es":
                return "spa";
            case "it":
                return "ita";
            case "ja":
                return "jpn";
            case "zh":
                return "zho";
            case "pt":
                return "por";
            case "ru":
                return "rus";
            default:
                return c;
        }
    }

    public static java.util.Set<Character> getExtensionKeys(Locale l) {
        return Collections.emptySet();
    }

    public static String getExtension(Locale l, char key) {
        return null;
    }

    public static java.util.Set<Character> getUnicodeLocaleAttributes(Locale l) {
        return Collections.emptySet();
    }

    public static java.util.Set<String> getUnicodeLocaleKeys(Locale l) {
        return Collections.emptySet();
    }

    public static String getUnicodeLocaleType(Locale l, String key) {
        return null;
    }

    public static boolean isSupported(String charsetName) {
        String n = charsetName.toUpperCase();
        return n.equals("UTF-8") || n.equals("UTF-16") || n.equals("ISO-8859-1") || n.equals("US-ASCII");
    }

    public static Locale forLanguageTag(String tag) {
        String[] p = tag.replace('_', '-').split("-");
        return p.length >= 2 ? new Locale(p[0], p[1]) : new Locale(p[0]);
    }
}
