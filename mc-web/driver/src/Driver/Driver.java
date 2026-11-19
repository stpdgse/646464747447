import java.io.File;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.ArrayList;
import java.util.List;
import org.teavm.backend.wasm.WasmDebugInfoLevel;
import org.teavm.dependency.DependencyInfo;
import org.teavm.model.ClassReader;
import org.teavm.tooling.TeaVMTool;
import org.teavm.tooling.TeaVMTargetType;
import org.teavm.tooling.TeaVMToolLog;

/**
 * Runs the TeaVM tool directly on the game so failures can be inspected in-process.
 * usage: Driver OUT_DIR MAIN_CLASS JAR...
 */
public class Driver {
    public static void main(String[] args) throws Exception {
        File out = new File(args[0]);
        String main = args[1];
        List<URL> urls = new ArrayList<>();
        List<File> files = new ArrayList<>();
        for (int i = 2; i < args.length; i++) {
            files.add(new File(args[i]));
            urls.add(new File(args[i]).toURI().toURL());
        }
        URLClassLoader cl = new URLClassLoader(urls.toArray(new URL[0]), Driver.class.getClassLoader());
        TeaVMTool tool = new TeaVMTool();
        tool.setTargetType(TeaVMTargetType.WEBASSEMBLY_GC);
        tool.setTargetDirectory(out);
        tool.setTargetFileName("game.wasm");
        tool.setMainClass(main);
        tool.setFastDependencyAnalysis(true);
        tool.setObfuscated(false);
        tool.setClassLoader(cl);
        tool.setClassPath(files);
        tool.setIncremental(false);
        tool.setLog(new TeaVMToolLog() {
            public void info(String s) {
                System.out.println("[teavm] " + s);
            }

            public void debug(String s) {
            }

            public void warning(String s) {
                System.out.println("[warn] " + s);
            }

            public void error(String s) {
                System.out.println("[error] " + s);
            }

            public void info(String s, Throwable t) {
                System.out.println("[teavm] " + s);
            }

            public void debug(String s, Throwable t) {
            }

            public void warning(String s, Throwable t) {
                System.out.println("[warn] " + s);
            }

            public void error(String s, Throwable t) {
                System.out.println("[error] " + s);
                t.printStackTrace(System.out);
            }
        });
        try {
            tool.generate();
            var problems = tool.getProblemProvider().getSevereProblems();
            if (problems.isEmpty()) {
                System.out.println("GENERATED OK");
            } else {
                System.out.println("(problems present)");
            }
            {
                var di = tool.getDependencyInfo();
                java.util.Map<String, Integer> pk = new java.util.TreeMap<>();
                for (String c : di.getReachableClasses()) {
                    String[] parts = c.split("\\.");
                    int n = Math.min(parts.length - 1, c.startsWith("net.minecraft") || c.startsWith("com.mojang") ? 4 : 3);
                    StringBuilder sb = new StringBuilder();
                    for (int i = 0; i < n; i++) {
                        sb.append(i > 0 ? "." : "").append(parts[i]);
                    }
                    pk.merge(sb.toString(), 1, Integer::sum);
                }
                for (var e : pk.entrySet()) {
                    System.out.println("PKG " + e.getValue() + " " + e.getKey());
                }
            }
            if (!problems.isEmpty()) {
                System.out.println("GENERATED WITH " + problems.size() + " SEVERE PROBLEMS");
                var consumer = new org.teavm.diagnostics.DefaultProblemTextConsumer();
                for (var p : problems) {
                    consumer.clear();
                    p.render(consumer);
                    System.out.println("PROBLEM: " + consumer.getText());
                    var loc = p.getLocation();
                    if (loc != null && loc.getMethod() != null) {
                        System.out.println("    at " + loc.getMethod());
                    }
                }
            }
        } catch (Throwable t) {
            System.out.println("GENERATE FAILED: " + t);
            for (Throwable c = t; c != null; c = c.getCause()) {
                System.out.println("  cause: " + c);
                StackTraceElement[] st = c.getStackTrace();
                for (int k = 0; k < Math.min(14, st.length); k++) {
                    System.out.println("    at " + st[k]);
                }
            }
            DependencyInfo info = tool.getDependencyInfo();
            if (info == null) {
                System.out.println("no dependency info");
                return;
            }
            var src = info.getClassSource();
            int bad = 0;
            for (String name : info.getReachableClasses()) {
                ClassReader c = src.get(name);
                if (c == null) {
                    System.out.println("reachable but no holder: " + name);
                    bad++;
                    continue;
                }
                if (c.getParent() != null && src.get(c.getParent()) == null) {
                    System.out.println("MISSING PARENT: " + c.getParent() + " of " + name);
                    bad++;
                }
                for (String i : c.getInterfaces()) {
                    if (src.get(i) == null) {
                        System.out.println("MISSING INTERFACE: " + i + " of " + name);
                        bad++;
                    }
                }
            }
            for (String name : info.getReachableClasses()) {
                ClassReader c = src.get(name);
                if (c != null && c.getParent() == null && !name.equals("java.lang.Object")
                        && !c.hasModifier(org.teavm.model.ElementModifier.INTERFACE)) {
                    System.out.println("ROOT CLASS (no parent, not interface): " + name);
                }
            }
            ClassReader obj = src.get("java.lang.Object");
            System.out.println("Object holder: " + obj + " methods=" + (obj == null ? -1 : obj.getMethods().size())
                    + " reachable=" + info.getReachableClasses().contains("java.lang.Object"));
            if (obj != null) {
                System.out.println("Object modifiers: " + obj.readModifiers() + " parent=" + obj.getParent()
                        + " interfaces=" + obj.getInterfaces());
                StringBuilder sb = new StringBuilder();
                for (var m : obj.getMethods()) {
                    sb.append(m.getName()).append(' ');
                }
                System.out.println("Object methods: " + sb);
            }
            try (var in = Driver.class.getClassLoader().getResourceAsStream("mcweb/stub-classes.txt")) {
                if (in != null) {
                    var r = new java.io.BufferedReader(new java.io.InputStreamReader(in));
                    String line;
                    java.util.Set<String> reach = new java.util.HashSet<>(info.getReachableClasses());
                    while ((line = r.readLine()) != null) {
                        String[] p = line.trim().split("\\s+");
                        if (p.length >= 2 && reach.contains(p[1])) {
                            ClassReader c = src.get(p[1]);
                            System.out.println("REACHABLE STUB: " + line.trim() + " methods="
                                    + (c == null ? -1 : c.getMethods().size())
                                    + " mods=" + (c == null ? "?" : c.readModifiers()));
                        }
                    }
                }
            }
            try {
                var names = new java.util.LinkedHashSet<>(info.getReachableClasses());
                var listable = new org.teavm.model.ListableClassReaderSource() {
                    public java.util.Set<String> getClassNames() {
                        return names;
                    }

                    public ClassReader get(String name) {
                        return src.get(name);
                    }
                };
                var vts = new org.teavm.backend.wasm.vtable.WasmGCVirtualTableProvider(listable,
                        info.getReachableMethods(), m -> true);
                var ot = vts.lookup("java.lang.Object");
                System.out.println("Object vtable: " + (ot == null ? "null" : ot.getClassName() + " used="
                        + ot.isUsed() + " concrete=" + ot.isConcrete()));
                int bad2 = 0;
                for (String name : info.getReachableClasses()) {
                    ClassReader cr = src.get(name);
                    var vt = vts.lookup(name);
                    if (cr != null && vt != null && !cr.hasModifier(org.teavm.model.ElementModifier.INTERFACE)
                            && !cr.hasModifier(org.teavm.model.ElementModifier.ABSTRACT) && !vt.isConcrete()
                            && bad2++ < 25) {
                        System.out.println("NONCONCRETE: " + name + " -> table of " + vt.getClassName());
                    }
                }
                System.out.println("non-concrete concrete classes: " + bad2);
                int merged = 0;
                for (String name : info.getReachableClasses()) {
                    var vt = vts.lookup(name);
                    if (vt != null && !vt.getClassName().equals(name) && merged++ < 8) {
                        System.out.println("  merged: " + name + " -> " + vt.getClassName());
                    }
                }
            } catch (Throwable ex) {
                System.out.println("vtable probe failed: " + ex);
            }
            System.out.println("checked " + info.getReachableClasses().size() + " classes, problems: " + bad);
        }
    }
}
