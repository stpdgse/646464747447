import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import com.mojang.brigadier.StringReader;
import com.mojang.brigadier.exceptions.CommandSyntaxException;
import com.mojang.serialization.JsonOps;
import net.minecraft.SharedConstants;
import java.util.ArrayList;
import java.util.Collections;
import net.minecraft.nbt.*;
import net.minecraft.nbt.NbtOps;
import net.minecraft.nbt.Tag;
import net.minecraft.nbt.TagParser;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.ComponentSerialization;
import net.minecraft.server.Bootstrap;

/**
 * Parses SNBT with the official parser. Input: one escaped line each (\n, \\ and \t unescaped).
 * Output per line: {@code OK <snbt>} for {@code parseFully} (the whole string),
 * {@code ARG <cursor> <snbt>} for the argument form that stops at the end of the value, or
 * {@code ERR <cursor> <json message>}; the two forms are printed as two lines.
 */
public class SnbtOracle {
    static String unescape(String s) {
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c == '\\' && i + 1 < s.length()) {
                char n = s.charAt(++i);
                b.append(n == 'n' ? '\n' : n == 't' ? '\t' : n);
            } else b.append(c);
        }
        return b.toString();
    }

    static String quote(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c >= 0x20 && c < 0x7f && c != '"' && c != '\\') b.append(c);
            else if (Character.isHighSurrogate(c) && i + 1 < s.length() && Character.isLowSurrogate(s.charAt(i + 1))) {
                b.append(String.format("\\u%04x\\u%04x", (int) c, (int) s.charAt(++i)));
            } else if (Character.isSurrogate(c)) b.append("\\ufffd"); // Rust strings cannot hold lone surrogates
            else b.append(String.format("\\u%04x", (int) c));
        }
        return b.append('"').toString();
    }

    /** Type-tagged, key-sorted form that the Rust side prints identically. */
    static String canon(Tag t) {
        if (t instanceof ByteTag b) return "B" + b.value();
        if (t instanceof ShortTag b) return "S" + b.value();
        if (t instanceof IntTag b) return "I" + b.value();
        if (t instanceof LongTag b) return "L" + b.value();
        if (t instanceof FloatTag b) return "F" + Integer.toHexString(Float.floatToRawIntBits(b.value()));
        if (t instanceof DoubleTag b) return "D" + Long.toHexString(Double.doubleToRawLongBits(b.value()));
        if (t instanceof StringTag b) return quote(b.value());
        if (t instanceof ByteArrayTag b) {
            StringBuilder o = new StringBuilder("[B;");
            byte[] a = b.getAsByteArray();
            for (int i = 0; i < a.length; i++) o.append(i > 0 ? "," : "").append(a[i]);
            return o.append(']').toString();
        }
        if (t instanceof IntArrayTag b) {
            StringBuilder o = new StringBuilder("[I;");
            int[] a = b.getAsIntArray();
            for (int i = 0; i < a.length; i++) o.append(i > 0 ? "," : "").append(a[i]);
            return o.append(']').toString();
        }
        if (t instanceof LongArrayTag b) {
            StringBuilder o = new StringBuilder("[L;");
            long[] a = b.getAsLongArray();
            for (int i = 0; i < a.length; i++) o.append(i > 0 ? "," : "").append(a[i]);
            return o.append(']').toString();
        }
        if (t instanceof ListTag b) {
            StringBuilder o = new StringBuilder("[");
            for (int i = 0; i < b.size(); i++) o.append(i > 0 ? "," : "").append(canon(b.get(i)));
            return o.append(']').toString();
        }
        CompoundTag c = (CompoundTag) t;
        ArrayList<String> keys = new ArrayList<>(c.keySet());
        Collections.sort(keys);
        StringBuilder o = new StringBuilder("{");
        boolean first = true;
        for (String k : keys) {
            if (!first) o.append(',');
            first = false;
            o.append(quote(k)).append(':').append(canon(c.get(k)));
        }
        return o.append('}').toString();
    }

    static String msg(CommandSyntaxException e) {
        Object raw = e.getRawMessage();
        if (raw instanceof Component c && c.getContents() instanceof net.minecraft.network.chat.contents.TranslatableContents tc) {
            StringBuilder b = new StringBuilder(tc.getKey());
            for (Object a : tc.getArgs()) b.append('\u0001').append(a instanceof Component ac ? ac.getString() : String.valueOf(a));
            return b.toString().replace("\n", "\\n");
        }
        return "lit:" + raw.toString();
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        List<String> lines = Files.readAllLines(Path.of(args[0]), StandardCharsets.UTF_8);
        TagParser<Tag> parser = TagParser.create(NbtOps.INSTANCE);
        try (PrintWriter w = new PrintWriter(args[1], StandardCharsets.UTF_8)) {
            for (String raw : lines) {
                String line = unescape(raw);
                try {
                    Tag t = parser.parseFully(line);
                    w.println("OK " + canon(t));
                } catch (CommandSyntaxException e) {
                    w.println("ERR " + e.getCursor() + " " + msg(e));
                } catch (Throwable t) {
                    w.println("CRASH " + t);
                }
                try {
                    StringReader r = new StringReader(line);
                    Tag t = parser.parseAsArgument(r);
                    w.println("ARG " + r.getCursor() + " " + canon(t));
                } catch (CommandSyntaxException e) {
                    w.println("ERR " + e.getCursor() + " " + msg(e));
                } catch (Throwable t) {
                    w.println("CRASH " + t);
                }
            }
        }
    }
}
