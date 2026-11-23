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
        System.out.println("[trace] " + message);
    }

    public static void traceValue(String label, Object value) {
        System.out.println("[trace] " + label + " = " + value);
    }

    // java.io / java.nio.file --------------------------------------------------------------------

    public static java.nio.file.Path toPath(java.io.File file) {
        return java.nio.file.Paths.get(file.getPath());
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
