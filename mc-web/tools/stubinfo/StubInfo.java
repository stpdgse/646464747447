import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.lang.reflect.Modifier;
import java.util.ArrayDeque;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;

/**
 * Reads placeholder class names (one per line on stdin) and prints their true kind, superclass,
 * abstractness and interfaces, followed by the same for every supertype (closure).
 *
 * Output line: KIND NAME PARENT FLAGS INTERFACES
 *   KIND = class | interface | exception ; PARENT = '-' for interfaces ; FLAGS = abstract | -
 *   INTERFACES = comma list or '-'
 * Classes that cannot be loaded are printed as "unknown NAME".
 */
public class StubInfo {
    public static void main(String[] args) throws Exception {
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        ArrayDeque<String> todo = new ArrayDeque<>();
        String l;
        while ((l = in.readLine()) != null) {
            if (!l.isBlank()) {
                todo.add(l.trim());
            }
        }
        Set<String> seen = new LinkedHashSet<>();
        Map<String, String> out = new LinkedHashMap<>();
        while (!todo.isEmpty()) {
            String name = todo.poll();
            if (!seen.add(name) || name.equals("java.lang.Object")) {
                continue;
            }
            Class<?> c;
            try {
                c = Class.forName(name, false, StubInfo.class.getClassLoader());
            } catch (Throwable e) {
                out.put(name, "unknown " + name);
                continue;
            }
            boolean itf = c.isInterface();
            boolean exc = Throwable.class.isAssignableFrom(c);
            String kind = itf ? "interface" : exc ? "exception" : "class";
            String parent = itf || c.getSuperclass() == null ? "-" : c.getSuperclass().getName();
            String flags = !itf && Modifier.isAbstract(c.getModifiers()) ? "abstract" : "-";
            StringBuilder ifs = new StringBuilder();
            for (Class<?> i : c.getInterfaces()) {
                ifs.append(ifs.length() > 0 ? "," : "").append(i.getName());
                todo.add(i.getName());
            }
            if (!itf && c.getSuperclass() != null) {
                todo.add(c.getSuperclass().getName());
            }
            out.put(name, kind + " " + name + " " + parent + " " + flags + " " + (ifs.length() == 0 ? "-" : ifs));
        }
        out.values().forEach(System.out::println);
    }
}
