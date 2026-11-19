package org.teavm.mcweb;

import java.util.ArrayList;
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
import org.teavm.model.MethodDescriptor;
import org.teavm.model.MethodHolder;
import org.teavm.model.MethodReference;
import org.teavm.model.Program;
import org.teavm.model.ValueType;
import org.teavm.model.Variable;
import org.teavm.model.instructions.ConstructInstruction;
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

    @Override
    public void transformClass(ClassHolder cls, ClassHolderTransformerContext context) {
        if (cls.getName().startsWith("org.teavm.") || cls.getName().startsWith("java.")) {
            return;
        }
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
                        rewrite((InvokeInstruction) insn, cls, context, generated);
                    }
                }
            }
        }
    }

    private void rewrite(InvokeInstruction invoke, ClassHolder cls, ClassHolderTransformerContext context,
            Map<MethodReference, MethodReference> generated) {
        MethodReference ref = invoke.getMethod();
        String owner = ref.getClassName();
        if (!owner.startsWith("java.") || ref.getName().equals("<init>") || ref.getName().equals("<clinit>")) {
            return;
        }
        var hierarchy = context.getHierarchy();
        if (hierarchy.getClassSource().get(owner) == null || hierarchy.resolve(ref) != null) {
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
