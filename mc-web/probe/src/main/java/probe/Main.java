package probe;

import java.security.MessageDigest;
import java.util.UUID;

public class Main {
    public static void main(String[] args) throws Exception {
        UUID u = new UUID(0x0123456789abcdefL, 0xfedcba9876543210L);
        System.out.println(u + " " + u.getMostSignificantBits());
        System.out.println(UUID.fromString("00000000-0000-0000-0000-000000000001").getLeastSignificantBits());
        MessageDigest md = MessageDigest.getInstance("SHA-1");
        byte[] d = md.digest("abc".getBytes("UTF-8"));
        StringBuilder sb = new StringBuilder();
        for (byte b : d) sb.append(String.format("%02x", b));
        System.out.println("sha1(abc)=" + sb);
        MessageDigest m5 = MessageDigest.getInstance("MD5");
        sb.setLength(0);
        for (byte b : m5.digest("abc".getBytes("UTF-8"))) sb.append(String.format("%02x", b));
        System.out.println("md5(abc)=" + sb);
        System.out.println("maxMemory=" + Runtime.getRuntime().maxMemory());
        System.out.println("env=" + System.getenv().size());
        System.out.println("fma=" + Math.fma(2f, 3f, 1f));
        System.out.println("script=[" + java.util.Locale.ENGLISH.getScript() + "]");
        System.out.println("pred=" + java.util.regex.Pattern.compile("b+").asPredicate().test("abbc"));
        System.out.println("tid=" + (Thread.currentThread().threadId() >= 0));
        try {
            System.out.println(Main.class.getModule());
        } catch (UnsupportedOperationException e) {
            System.out.println("expected: " + e.getMessage());
        }
        System.out.println("fine=" + java.util.logging.Level.FINE + " utf8=" + java.nio.charset.StandardCharsets.UTF_8
                + " maxint=" + Integer.MAX_VALUE + " empty=" + java.util.Collections.EMPTY_LIST);
        MessageDigest s2 = MessageDigest.getInstance("SHA-256");
        sb.setLength(0);
        for (byte b : s2.digest("abc".getBytes("UTF-8"))) sb.append(String.format("%02x", b));
        System.out.println("sha256(abc)=" + sb);
    }
}
