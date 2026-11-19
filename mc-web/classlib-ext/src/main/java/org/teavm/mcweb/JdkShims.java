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

    // java.io ----------------------------------------------------------------------------------

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
        return null;
    }

    public static java.net.URL getSystemResource(String name) {
        return null;
    }

    public static Enumeration<java.net.URL> getResources(ClassLoader cl, String name) {
        return Collections.emptyEnumeration();
    }

    public static Enumeration<java.net.URL> getSystemResources(String name) {
        return Collections.emptyEnumeration();
    }

    public static java.net.URL getResource(Class<?> c, String name) {
        return null;
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
