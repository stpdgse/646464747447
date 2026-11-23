package probe;

import com.google.common.collect.MapMaker;
import com.google.common.collect.Maps;
import java.util.concurrent.ConcurrentMap;

public class GuavaProbe {
    record Key(String a, String b) { }

    static class Sleepy implements java.util.function.Function<String, Object> {
        @Override
        public Object apply(String s) {
            try {
                Thread.sleep(5);
            } catch (InterruptedException e) {
                throw new RuntimeException(e);
            }
            return s;
        }
    }

    static final ConcurrentMap<Key, Object> VALUES = new MapMaker().weakValues().makeMap();

    static Object create(String a, String b) {
        return VALUES.computeIfAbsent(new Key(a, b), k -> new Object[] {k.a(), k.b()});
    }

    public static void main(String[] args) {
        System.out.println("map class: " + VALUES.getClass().getName());
        VALUES.put(new Key("m", "n"), "direct");
        System.out.println("weak get: " + VALUES.get(new Key("m", "n")));
        System.out.println("weak putIfAbsent: " + VALUES.putIfAbsent(new Key("r", "s"), "first") + " then " + VALUES.get(new Key("r", "s")));
        System.out.println("create1: " + java.util.Arrays.toString((Object[]) create("p", "q")));
        System.out.println("create2: " + java.util.Arrays.toString((Object[]) create("p", "q")));
        System.out.println("sleepy: " + new Sleepy().apply("z"));
        ConcurrentMap<Key, String> m = Maps.newConcurrentMap();
        System.out.println("class: " + m.getClass().getName());
        System.out.println("get: " + m.get(new Key("a", "b")));
        System.out.println("putIfAbsent: " + m.putIfAbsent(new Key("a", "b"), "v1"));
        System.out.println("get2: " + m.get(new Key("a", "b")));
        System.out.println("computeIfAbsent(new): " + m.computeIfAbsent(new Key("c", "d"), k -> "made-" + k.a()));
        System.out.println("computeIfAbsent(old): " + m.computeIfAbsent(new Key("c", "d"), k -> "again"));
        System.out.println("size: " + m.size());
        ConcurrentMap<String, Object> m2 = Maps.newConcurrentMap();
        System.out.println("put/get: " + m2.put("x", "y") + " " + m2.get("x"));
    }
}
