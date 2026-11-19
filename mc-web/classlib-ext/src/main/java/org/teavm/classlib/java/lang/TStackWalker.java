package org.teavm.classlib.java.lang;

import java.util.Collections;
import java.util.Set;
import java.util.function.Function;
import java.util.stream.Stream;

public final class TStackWalker extends TObject {
    public enum Option {
        RETAIN_CLASS_REFERENCE, SHOW_REFLECT_FRAMES, SHOW_HIDDEN_FRAMES
    }

    private TStackWalker() {
    }

    public static TStackWalker getInstance() {
        return new TStackWalker();
    }

    public static TStackWalker getInstance(Option option) {
        return new TStackWalker();
    }

    public static TStackWalker getInstance(Set<Option> options) {
        return new TStackWalker();
    }

    public static TStackWalker getInstance(Set<Option> options, int estimateDepth) {
        return new TStackWalker();
    }

    public <T> T walk(Function<? super Stream<StackFrame>, ? extends T> function) {
        return function.apply(Collections.<StackFrame>emptyList().stream());
    }

    public interface StackFrame {
        String getClassName();

        String getMethodName();
    }
}
