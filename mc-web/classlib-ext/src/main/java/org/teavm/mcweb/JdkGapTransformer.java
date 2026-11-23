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
    /** Methods TeaVM implements but whose behaviour the browser port replaces. */
    private static final Set<String> FORCED = Set.of(
            "java.lang.Class#getResourceAsStream",
            "java.lang.Class#getGenericSuperclass",
            "java.lang.Class#getGenericInterfaces",
            "java.lang.Class#isAnonymousClass",
            "java.lang.ClassLoader#getResourceAsStream",
            "java.lang.ClassLoader#getSystemResourceAsStream",
            "java.nio.file.Files#createDirectories",
            "java.nio.file.Paths#get",
            "java.nio.file.Path#of",
            "java.lang.Class#getResource",
            "java.lang.ClassLoader#getResource",
            "java.lang.ClassLoader#getResources",
            "java.lang.ClassLoader#getSystemResource",
            "java.lang.ClassLoader#getSystemResources");
    private final java.util.Map<String, String> stubClasses = new HashMap<>();
    private final List<String> lenientPrefixes = new ArrayList<>();
    private final Set<String> cutMethods = new HashSet<>();
    private final Set<String> noopMethods = new HashSet<>();
    private final Map<String, String> redirectMethods = new HashMap<>();
    private final Set<String> traceClasses = new HashSet<>();
    private final List<String[]> traceFields = new ArrayList<>();
    private final Set<String> traceMethods = new HashSet<>();
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
                if (kind.equals("tracem")) {
                    // tracem <class> <method>: print on entry of every overload (debugging aid)
                    traceMethods.add(name + "#" + p[2]);
                    continue;
                }
                if (kind.equals("tracefield")) {
                    // tracefield <class> <field>: print the field's value when the class initializer ends
                    traceFields.add(new String[] {name, p[2]});
                    traceClasses.add(name);
                    continue;
                }
                if (kind.equals("trace")) {
                    // trace <class>: print when its static initializer starts and finishes (debugging aid)
                    traceClasses.add(name);
                    continue;
                }
                if (kind.equals("redirect")) {
                    // redirect <class> <method> <shimMethod>: body becomes a call to JdkShims.<shimMethod>(same params)
                    redirectMethods.put(name + "#" + p[2], p[3]);
                    continue;
                }
                if (kind.equals("noop")) {
                    // noop <class> <method>: replace every overload (void methods/constructors) with an empty body
                    noopMethods.add(name + "#" + p[2]);
                    continue;
                }
                if (kind.equals("cut")) {
                    // cut <class> <method>: replace every overload's body with a "not available" error
                    cutMethods.add(name + "#" + p[2]);
                    continue;
                }
                if (context.getHierarchy().getClassSource().get(name) != null) {
                    continue;
                }
                String parent = p.length > 2 && !p[2].equals("-") ? p[2]
                        : kind.equals("exception") ? "java.io.IOException" : "java.lang.Object";
                boolean isAbstract = p.length > 3 && p[3].equals("abstract");
                String[] interfaces = p.length > 4 && !p[4].equals("-") ? p[4].split(",") : new String[0];
                context.submit(createStubClass(kind, name, parent, isAbstract, interfaces));
                stubClasses.put(name, kind.equals("interface") ? "java.lang.Object" : parent);
            }
        } catch (java.io.IOException e) {
            throw new IllegalStateException(e);
        }
    }

    private static ClassHolder createStubClass(String kind, String name, String parent, boolean isAbstract,
            String[] interfaces) {
        ClassHolder c = new ClassHolder(name);
        c.setLevel(AccessLevel.PUBLIC);
        for (String i : interfaces) {
            c.getInterfaces().add(i);
        }
        if (kind.equals("interface")) {
            c.getModifiers().add(ElementModifier.INTERFACE);
            c.getModifiers().add(ElementModifier.ABSTRACT);
            c.setParent("java.lang.Object");
            return c;
        }
        if (isAbstract) {
            c.getModifiers().add(ElementModifier.ABSTRACT);
        }
        c.setParent(parent);
        return c;
    }

    @Override
    public void transformClass(ClassHolder cls, ClassHolderTransformerContext context) {
        loadStubs(context);
        if (cls.getName().startsWith("org.teavm.runtime.Fiber$Platform")) {
            // These classes are really browser objects (anyref). With reflection support enabled TeaVM
            // would generate a reflective constructor helper for their public default constructor, and
            // that helper is ill-typed. Hide the constructor from reflection.
            for (MethodHolder m : cls.getMethods()) {
                if (m.getName().equals("<init>")) {
                    m.setLevel(AccessLevel.PACKAGE_PRIVATE);
                }
            }
            return;
        }
        if (cls.getName().equals("org.teavm.runtime.Fiber")) {
            redirectFiberArrayGrowth(cls);
            return;
        }
        if (cls.getName().startsWith("org.teavm.")) {
            return;
        }
        // TeaVM's own class library shows up under java.* names. Never touch it with generated
        // stubs (its internal references resolve differently); only explicit real shims apply.
        boolean libraryCaller = cls.getName().startsWith("java.") || cls.getName().startsWith("javax.");
        Map<MethodReference, MethodReference> generated = new HashMap<>();
        if (!keepsMonitors(cls.getName())) {
            stripMonitors(cls);
        }
        for (MethodHolder m : cls.getMethods()) {
            if (traceMethods.contains(cls.getName() + "#" + m.getName()) && m.getProgram() != null) {
                Program mp = m.getProgram();
                InvokeInstruction enter = traceCall(mp, "enter " + cls.getName() + "." + m.getName()
                        + m.getDescriptor());
                StringConstantInstruction text = pendingConstant;
                BasicBlock first = mp.basicBlockAt(0);
                first.addFirst(enter);
                first.addFirst(text);
                if (m.getResultType() instanceof ValueType.Object) {
                    for (int bi = 0; bi < mp.basicBlockCount(); ++bi) {
                        for (Instruction insn : blockInstructions(mp.basicBlockAt(bi))) {
                            if (insn instanceof org.teavm.model.instructions.ExitInstruction
                                    && ((org.teavm.model.instructions.ExitInstruction) insn).getValueToReturn() != null) {
                                Variable label = mp.createVariable();
                                StringConstantInstruction labelText = new StringConstantInstruction();
                                labelText.setConstant("return " + cls.getName() + "." + m.getName());
                                labelText.setReceiver(label);
                                InvokeInstruction show = new InvokeInstruction();
                                show.setType(InvocationType.SPECIAL);
                                show.setMethod(new MethodReference(SHIMS, "traceValue",
                                        ValueType.object("java.lang.String"), ValueType.object("java.lang.Object"),
                                        ValueType.VOID));
                                show.setArguments(label,
                                        ((org.teavm.model.instructions.ExitInstruction) insn).getValueToReturn());
                                insn.insertPrevious(labelText);
                                insn.insertPrevious(show);
                            }
                        }
                    }
                }
            }
        }
        if (traceClasses.contains(cls.getName())) {
            MethodHolder clinit = cls.getMethod(new MethodDescriptor("<clinit>", ValueType.VOID));
            if (clinit != null && clinit.getProgram() != null) {
                instrumentClinit(clinit, cls.getName());
            }
        }
        for (MethodHolder method : new ArrayList<>(cls.getMethods())) {
            String redirectTo = redirectMethods.get(cls.getName() + "#" + method.getName());
            if (redirectTo != null && method.getProgram() != null) {
                Program redirected = new Program();
                redirected.createVariable();
                boolean isStatic = method.hasModifier(ElementModifier.STATIC);
                int paramCount = method.getParameterTypes().length;
                List<Variable> params = new ArrayList<>();
                for (int pi = 0; pi < paramCount; pi++) {
                    params.add(redirected.createVariable());
                }
                BasicBlock rb = redirected.createBasicBlock();
                InvokeInstruction call = new InvokeInstruction();
                call.setType(InvocationType.SPECIAL);
                call.setMethod(new MethodReference(SHIMS, redirectTo, method.getSignature()));
                call.setArguments(params.toArray(new Variable[0]));
                org.teavm.model.instructions.ExitInstruction rexit = new org.teavm.model.instructions.ExitInstruction();
                if (method.getResultType() != ValueType.VOID) {
                    Variable result = redirected.createVariable();
                    call.setReceiver(result);
                    rb.add(call);
                    rexit.setValueToReturn(result);
                } else {
                    rb.add(call);
                }
                rb.add(rexit);
                method.setProgram(redirected);
                continue;
            }
            if (noopMethods.contains(cls.getName() + "#" + method.getName()) && method.getProgram() != null
                    && method.getResultType() == ValueType.VOID) {
                Program empty = new Program();
                empty.createVariable();
                for (int pi = 0; pi < method.getParameterTypes().length; pi++) {
                    empty.createVariable();
                }
                BasicBlock block0 = empty.createBasicBlock();
                block0.add(new org.teavm.model.instructions.ExitInstruction());
                method.setProgram(empty);
                continue;
            }
            if (cutMethods.contains(cls.getName() + "#" + method.getName()) && method.getProgram() != null) {
                MethodHolder t = createThrowing(method.getName(), method.getSignature(),
                        cls.getName() + "." + method.getName() + " was cut from the browser build");
                method.setProgram(t.getProgram());
                continue;
            }
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
        if (FORCED.contains(owner + "#" + ref.getName()) && !cls.getName().startsWith("org.teavm.")) {
            List<ValueType> sigF = new ArrayList<>();
            if (invoke.getInstance() != null) {
                sigF.add(ValueType.object(owner));
            }
            for (ValueType t : ref.getDescriptor().getParameterTypes()) {
                sigF.add(t);
            }
            sigF.add(ref.getDescriptor().getResultType());
            MethodDescriptor forcedDesc = new MethodDescriptor(ref.getName(), sigF.toArray(new ValueType[0]));
            var shimsF = context.getHierarchy().getClassSource().get(SHIMS);
            if (shimsF != null && shimsF.getMethod(forcedDesc) != null) {
                List<Variable> argsF = new ArrayList<>();
                if (invoke.getInstance() != null) {
                    argsF.add(invoke.getInstance());
                }
                argsF.addAll(invoke.getArguments());
                invoke.setType(InvocationType.SPECIAL);
                invoke.setInstance(null);
                invoke.setMethod(new MethodReference(SHIMS, forcedDesc));
                invoke.setArguments(argsF.toArray(new Variable[0]));
                return;
            }
        }
        if (owner.equals("java.lang.Object") && invoke.getInstance() != null
                && (ref.getName().equals("wait") || ref.getName().equals("notify")
                || ref.getName().equals("notifyAll"))) {
            // Monitors are gone, so wait/notify become a cooperative poll (spurious wakeups are legal).
            List<ValueType> sig2 = new ArrayList<>();
            sig2.add(ValueType.object("java.lang.Object"));
            for (ValueType t : ref.getDescriptor().getParameterTypes()) {
                sig2.add(t);
            }
            sig2.add(ValueType.VOID);
            List<Variable> args2 = new ArrayList<>();
            args2.add(invoke.getInstance());
            args2.addAll(invoke.getArguments());
            invoke.setType(InvocationType.SPECIAL);
            invoke.setInstance(null);
            invoke.setMethod(new MethodReference(SHIMS, ref.getName(), sig2.toArray(new ValueType[0])));
            invoke.setArguments(args2.toArray(new Variable[0]));
            return;
        }
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

    /** Fiber.push grows its arrays with Arrays.copyOf, which is a coroutine here; see {@link FiberArrays}. */
    private static void redirectFiberArrayGrowth(ClassHolder fiber) {
        for (MethodHolder m : fiber.getMethods()) {
            Program program = m.getProgram();
            if (program == null) {
                continue;
            }
            for (int i = 0; i < program.basicBlockCount(); ++i) {
                for (Instruction insn : blockInstructions(program.basicBlockAt(i))) {
                    if (insn instanceof InvokeInstruction) {
                        InvokeInstruction call = (InvokeInstruction) insn;
                        MethodReference ref = call.getMethod();
                        if (ref.getClassName().equals("java.util.Arrays") && ref.getName().equals("copyOf")
                                && ref.parameterCount() == 2) {
                            call.setMethod(new MethodReference("org.teavm.mcweb.FiberArrays", ref.getDescriptor()));
                        }
                    }
                }
            }
        }
    }

    /**
     * The browser build has one thread of execution with cooperative fibers, so monitors protect
     * nothing. Worse, TeaVM treats {@code synchronized} as a potential suspension point, which spreads
     * "async" through the call graph until even static initialisers become coroutines and run
     * out of order. Monitors are therefore removed everywhere except TeaVM's own scheduler.
     */
    private static boolean keepsMonitors(String className) {
        return className.startsWith("org.teavm.") || className.equals("java.lang.Object")
                || className.equals("java.lang.Thread") || className.startsWith("java.lang.Thread$");
    }

    private static void stripMonitors(ClassHolder cls) {
        for (MethodHolder m : cls.getMethods()) {
            m.getModifiers().remove(ElementModifier.SYNCHRONIZED);
            Program program = m.getProgram();
            if (program == null) {
                continue;
            }
            for (int i = 0; i < program.basicBlockCount(); ++i) {
                for (Instruction insn : blockInstructions(program.basicBlockAt(i))) {
                    if (insn instanceof org.teavm.model.instructions.MonitorEnterInstruction
                            || insn instanceof org.teavm.model.instructions.MonitorExitInstruction) {
                        org.teavm.model.instructions.EmptyInstruction empty =
                                new org.teavm.model.instructions.EmptyInstruction();
                        empty.setLocation(insn.getLocation());
                        insn.replace(empty);
                    }
                }
            }
        }
    }

    private static InvokeInstruction traceCall(Program program, String message) {
        Variable text = program.createVariable();
        StringConstantInstruction str = new StringConstantInstruction();
        str.setConstant(message);
        str.setReceiver(text);
        InvokeInstruction call = new InvokeInstruction();
        call.setType(InvocationType.SPECIAL);
        call.setMethod(new MethodReference(SHIMS, "trace", ValueType.object("java.lang.String"), ValueType.VOID));
        call.setArguments(text);
        // the constant must precede the call: chain them through a holder list
        call.setLocation(null);
        pendingConstant = str;
        return call;
    }

    private static StringConstantInstruction pendingConstant;

    private void instrumentClinit(MethodHolder clinit, String className) {
        Program program = clinit.getProgram();
        BasicBlock first = program.basicBlockAt(0);
        InvokeInstruction enter = traceCall(program, "<clinit> start " + className);
        StringConstantInstruction enterText = pendingConstant;
        first.addFirst(enter);
        first.addFirst(enterText);
        for (int i = 0; i < program.basicBlockCount(); ++i) {
            BasicBlock block = program.basicBlockAt(i);
            for (Instruction insn : new ArrayList<Instruction>(blockInstructions(block))) {
                if (insn instanceof org.teavm.model.instructions.ExitInstruction) {
                    InvokeInstruction leave = traceCall(program, "<clinit> end " + className);
                    StringConstantInstruction leaveText = pendingConstant;
                    insn.insertPrevious(leaveText);
                    insn.insertPrevious(leave);
                    for (String[] tf : traceFields) {
                        if (tf[0].equals(className)) {
                            Variable value = program.createVariable();
                            GetFieldInstruction get = new GetFieldInstruction();
                            get.setField(new FieldReference(className, tf[1]));
                            get.setFieldType(ValueType.object("java.lang.Object"));
                            get.setReceiver(value);
                            InvokeInstruction show = new InvokeInstruction();
                            show.setType(InvocationType.SPECIAL);
                            show.setMethod(new MethodReference(SHIMS, "traceValue", ValueType.object("java.lang.String"),
                                    ValueType.object("java.lang.Object"), ValueType.VOID));
                            Variable label = program.createVariable();
                            StringConstantInstruction labelText = new StringConstantInstruction();
                            labelText.setConstant(className + "." + tf[1]);
                            labelText.setReceiver(label);
                            show.setArguments(label, value);
                            insn.insertPrevious(labelText);
                            insn.insertPrevious(get);
                            insn.insertPrevious(show);
                        }
                    }
                }
            }
        }
    }

    private static List<Instruction> blockInstructions(BasicBlock block) {
        List<Instruction> out = new ArrayList<>();
        for (Instruction i : block) {
            out.add(i);
        }
        return out;
    }

    private boolean isLenient(String owner) {
        for (String p : lenientPrefixes) {
            if (owner.startsWith(p)) {
                return true;
            }
        }
        return false;
    }

    private boolean isPlatform(String owner) {
        return owner.startsWith("java.") || owner.startsWith("javax.") || stubClasses.containsKey(owner)
                || isLenient(owner);
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
        // Only placeholders (and library classes inheriting from them) get the null/0 fallback. Real
        // JDK classes never do: the hierarchy lookup does not see TeaVM's mapped fields reliably, so a
        // "missing" field there is usually present (Level.FINE, StandardCharsets.UTF_8, ...).
        String owner = f.getClassName();
        // Only for classes that are placeholders (or inherit from one), or that do not exist at all.
        // A real class is never touched: field lookups on it are not reliable enough to call a field "missing".
        boolean placeholder = stubClasses.containsKey(owner) || inheritsStub(owner, context)
                || (!isResolvable(owner, context) && (isPlatform(owner) || isLenient(owner)));
        if (!placeholder || (isResolvable(owner, context) && isFieldResolved(f, context))) {
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

        // Log a real stack trace first: TeaVM exception stack traces carry no frames in this build.
        Variable hitText = program.createVariable();
        StringConstantInstruction hitStr = new StringConstantInstruction();
        hitStr.setConstant(message);
        hitStr.setReceiver(hitText);
        block.add(hitStr);
        InvokeInstruction hit = new InvokeInstruction();
        hit.setType(InvocationType.SPECIAL);
        hit.setMethod(new MethodReference(SHIMS, "gapHit", ValueType.object("java.lang.String"), ValueType.VOID));
        hit.setArguments(hitText);
        block.add(hit);

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
