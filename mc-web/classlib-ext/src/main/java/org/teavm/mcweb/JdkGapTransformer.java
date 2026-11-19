package org.teavm.mcweb;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.Set;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.teavm.model.AccessLevel;
import org.teavm.model.BasicBlock;
import org.teavm.model.ClassHolder;
import org.teavm.model.ClassHolderTransformer;
import org.teavm.model.ClassHolderTransformerContext;
import org.teavm.model.ElementModifier;
import org.teavm.model.Instruction;
import org.teavm.model.ClassReader;
import org.teavm.model.FieldReference;
import org.teavm.model.PrimitiveType;
import org.teavm.model.MethodDescriptor;
import org.teavm.model.MethodHolder;
import org.teavm.model.MethodReference;
import org.teavm.model.Program;
import org.teavm.model.ValueType;
import org.teavm.model.Variable;
import org.teavm.model.instructions.ConstructInstruction;
import org.teavm.model.instructions.GetFieldInstruction;
import org.teavm.model.instructions.IntegerConstantInstruction;
import org.teavm.model.instructions.NullConstantInstruction;
import org.teavm.model.instructions.ExitInstruction;
import org.teavm.model.instructions.InvocationType;
import org.teavm.model.instructions.InvokeInstruction;
import org.teavm.model.instructions.RaiseInstruction;
import org.teavm.model.instructions.StringConstantInstruction;

/**
 * Finds calls to JDK methods that TeaVM's class library does not implement and redirects them.
 *
 * <ol>
 *   <li>If {@link JdkShims} has a static method with the same name whose first parameter is the
 *       receiver (or the same parameters for static calls), the call goes there.</li>
 *   <li>Otherwise the call goes to a generated static method that throws
 *       {@link UnsupportedOperationException} naming the missing JDK method. This turns a hard
 *       compile error into a clear runtime error on the (usually cold) path that needs it.</li>
 * </ol>
 */
public class JdkGapTransformer implements ClassHolderTransformer {
    private static final String SHIMS = "org.teavm.mcweb.JdkShims";
    private final java.util.Map<String, String> stubClasses = new HashMap<>();
    private final List<String> lenientPrefixes = new ArrayList<>();
    private boolean stubsLoaded;

    /**
     * Declares placeholder classes for platform types the browser has no implementation of
     * (sockets, selectors, crypto, ...). Format of mcweb/stub-classes.txt, one per line:
     * {@code class|interface|exception <name> [superclass]}. Every member access on a placeholder
     * becomes a generated "not available" stub, so linking succeeds and only real use fails.
     */
    private void loadStubs(ClassHolderTransformerContext context) {
        if (stubsLoaded) {
            return;
        }
        stubsLoaded = true;
        try (var in = JdkGapTransformer.class.getClassLoader().getResourceAsStream("mcweb/stub-classes.txt")) {
            if (in == null) {
                return;
            }
            BufferedReader r = new BufferedReader(new InputStreamReader(in, StandardCharsets.UTF_8));
            String line;
            while ((line = r.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#")) {
                    continue;
                }
                String[] p = line.split("\\s+");
                String kind = p[0];
                String name = p[1];
                if (kind.equals("lenient")) {
                    lenientPrefixes.add(name);
                    continue;
                }
                if (context.getHierarchy().getClassSource().get(name) != null) {
                    continue;
                }
                String parent = p.length > 2 ? p[2] : kind.equals("exception") ? "java.io.IOException"
                        : "java.lang.Object";
                context.submit(createStubClass(kind, name, parent));
                stubClasses.put(name, kind.equals("interface") ? "java.lang.Object" : parent);
            }
        } catch (java.io.IOException e) {
            throw new IllegalStateException(e);
        }
    }

    private static ClassHolder createStubClass(String kind, String name, String parent) {
        ClassHolder c = new ClassHolder(name);
        c.setLevel(AccessLevel.PUBLIC);
        if (kind.equals("interface")) {
            c.getModifiers().add(ElementModifier.INTERFACE);
            c.getModifiers().add(ElementModifier.ABSTRACT);
            c.setParent("java.lang.Object");
            // A method-less interface lets TeaVM's WASM vtable builder fold it into java.lang.Object's
            // table and corrupt Object's "concrete" flag, so give every placeholder one marker method.
            MethodHolder marker = new MethodHolder("mcweb$marker", ValueType.VOID);
            marker.setLevel(AccessLevel.PUBLIC);
            marker.getModifiers().add(ElementModifier.ABSTRACT);
            c.addMethod(marker);
            return c;
        }
        c.setParent(parent);
        return c;
    }

    @Override
    public void transformClass(ClassHolder cls, ClassHolderTransformerContext context) {
        loadStubs(context);
        if (cls.getName().startsWith("org.teavm.")) {
            return;
        }
        // TeaVM's own class library shows up under java.* names. Never touch it with generated
        // stubs (its internal references resolve differently); only explicit real shims apply.
        boolean libraryCaller = cls.getName().startsWith("java.") || cls.getName().startsWith("javax.");
        Map<MethodReference, MethodReference> generated = new HashMap<>();
        for (MethodHolder method : new ArrayList<>(cls.getMethods())) {
            Program program = method.getProgram();
            if (program == null) {
                continue;
            }
            for (int i = 0; i < program.basicBlockCount(); ++i) {
                BasicBlock block = program.basicBlockAt(i);
                for (Instruction insn : block) {
                    if (insn instanceof InvokeInstruction) {
                        rewrite((InvokeInstruction) insn, cls, context, generated, libraryCaller);
                    } else if (!libraryCaller && insn instanceof GetFieldInstruction) {
                        rewriteField((GetFieldInstruction) insn, context);
                    }
                }
            }
        }
    }

    private void rewrite(InvokeInstruction invoke, ClassHolder cls, ClassHolderTransformerContext context,
            Map<MethodReference, MethodReference> generated, boolean libraryCaller) {
        MethodReference ref = invoke.getMethod();
        String owner = ref.getClassName();
        if (ref.getName().equals("<clinit>") || !(isPlatform(owner) || inheritsStub(owner, context))) {
            return;
        }
        var hierarchy = context.getHierarchy();
        if (!isResolvable(owner, context) || isResolved(ref.getClassName(), ref.getDescriptor(), context)) {
            return;
        }
        boolean instanceCall = invoke.getInstance() != null;

        ValueType[] origParams = ref.getDescriptor().getParameterTypes();
        List<ValueType> sig = new ArrayList<>();
        if (instanceCall) {
            sig.add(ValueType.object(owner));
        }
        for (ValueType t : origParams) {
            sig.add(t);
        }
        sig.add(ref.getDescriptor().getResultType());
        MethodDescriptor shimDesc = new MethodDescriptor(ref.getName(), sig.toArray(new ValueType[0]));

        MethodReference target;
        var shims = hierarchy.getClassSource().get(SHIMS);
        if (shims != null && shims.getMethod(shimDesc) != null) {
            target = new MethodReference(SHIMS, shimDesc);
        } else if (libraryCaller) {
            return;
        } else {
            target = generated.computeIfAbsent(new MethodReference(cls.getName(), shimDesc), k -> {
                String name = "mcweb$missing$" + ref.getClassName().replace('.', '_') + "$" + ref.getName()
                        + "$" + generated.size();
                MethodHolder h = createThrowing(name, sig.toArray(new ValueType[0]),
                        ref.getClassName() + "." + ref.getName() + " is not available in the browser port");
                cls.addMethod(h);
                return h.getReference();
            });
        }

        List<Variable> args = new ArrayList<>();
        if (instanceCall) {
            args.add(invoke.getInstance());
        }
        args.addAll(invoke.getArguments());
        invoke.setType(InvocationType.SPECIAL);
        invoke.setInstance(null);
        invoke.setMethod(target);
        invoke.setArguments(args.toArray(new Variable[0]));
    }

    private boolean isResolvable(String owner, ClassHolderTransformerContext context) {
        return stubClasses.containsKey(owner) || context.getHierarchy().getClassSource().get(owner) != null;
    }

    /** True when the method exists on the class, a superclass or, for placeholders, its declared parent. */
    private boolean isResolved(String owner, MethodDescriptor desc, ClassHolderTransformerContext context) {
        var h = context.getHierarchy();
        String start = owner;
        if (stubClasses.containsKey(owner)) {
            if (h.resolve("java.lang.Object", desc) != null) {
                return true;
            }
            start = stubClasses.get(owner);
            if (desc.getName().equals("<init>")) {
                return false;
            }
        }
        return h.getClassSource().get(start) != null && h.resolve(start, desc) != null;
    }

    private boolean isFieldResolved(FieldReference f, ClassHolderTransformerContext context) {
        if (stubClasses.containsKey(f.getClassName())) {
            return false;
        }
        return context.getHierarchy().resolve(f) != null;
    }

    private boolean isPlatform(String owner) {
        if (owner.startsWith("java.") || owner.startsWith("javax.") || stubClasses.containsKey(owner)) {
            return true;
        }
        for (String p : lenientPrefixes) {
            if (owner.startsWith(p)) {
                return true;
            }
        }
        return false;
    }

    /** Library classes that extend a placeholder (e.g. netty's JFR events) inherit its missing members. */
    private boolean inheritsStub(String owner, ClassHolderTransformerContext context) {
        var src = context.getHierarchy().getClassSource();
        int guard = 0;
        for (var c = src.get(owner); c != null && guard++ < 32; c = c.getParent() == null ? null : src.get(c.getParent())) {
            if (c.getParent() != null && stubClasses.containsKey(c.getParent())) {
                return true;
            }
        }
        return false;
    }

    private void rewriteField(GetFieldInstruction get, ClassHolderTransformerContext context) {
        FieldReference f = get.getField();
        if (!isPlatform(f.getClassName())
                || (isResolvable(f.getClassName(), context) && isFieldResolved(f, context))) {
            return;
        }
        Instruction replacement;
        ValueType t = get.getFieldType();
        if (t instanceof ValueType.Primitive) {
            IntegerConstantInstruction c = new IntegerConstantInstruction();
            c.setConstant(0);
            c.setReceiver(get.getReceiver());
            replacement = c;
        } else {
            NullConstantInstruction c = new NullConstantInstruction();
            c.setReceiver(get.getReceiver());
            replacement = c;
        }
        replacement.setLocation(get.getLocation());
        get.replace(replacement);
    }

    private static MethodHolder createThrowing(String name, ValueType[] signature, String message) {
        MethodHolder holder = new MethodHolder(name, signature);
        holder.getModifiers().add(ElementModifier.STATIC);
        holder.setLevel(AccessLevel.PUBLIC);

        Program program = new Program();
        program.createVariable(); // slot 0 is reserved
        for (int i = 0; i < signature.length - 1; i++) {
            program.createVariable();
        }
        BasicBlock block = program.createBasicBlock();

        Variable ex = program.createVariable();
        ConstructInstruction construct = new ConstructInstruction();
        construct.setType("java.lang.UnsupportedOperationException");
        construct.setReceiver(ex);
        block.add(construct);

        Variable msg = program.createVariable();
        StringConstantInstruction str = new StringConstantInstruction();
        str.setConstant(message);
        str.setReceiver(msg);
        block.add(str);

        InvokeInstruction init = new InvokeInstruction();
        init.setType(InvocationType.SPECIAL);
        init.setMethod(new MethodReference("java.lang.UnsupportedOperationException", "<init>",
                ValueType.object("java.lang.String"), ValueType.VOID));
        init.setInstance(ex);
        init.setArguments(msg);
        block.add(init);

        RaiseInstruction raise = new RaiseInstruction();
        raise.setException(ex);
        block.add(raise);

        holder.setProgram(program);
        return holder;
    }
}
