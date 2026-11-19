package org.teavm.classlib.java.lang;

/** Scoped values on a single logical stack: a plain stack of bindings per value. */
public final class TScopedValue<T> extends TObject {
    private final java.util.ArrayDeque<T> bindings = new java.util.ArrayDeque<>();

    private TScopedValue() {
    }

    public static <T> TScopedValue<T> newInstance() {
        return new TScopedValue<>();
    }

    public T get() {
        if (bindings.isEmpty()) {
            throw new java.util.NoSuchElementException();
        }
        return bindings.peek();
    }

    public boolean isBound() {
        return !bindings.isEmpty();
    }

    public T orElse(T other) {
        return bindings.isEmpty() ? other : bindings.peek();
    }

    public static <T> Carrier where(TScopedValue<T> key, T value) {
        return new Carrier().where(key, value);
    }

    public static final class Carrier {
        private final java.util.List<TScopedValue<Object>> keys = new java.util.ArrayList<>();
        private final java.util.List<Object> values = new java.util.ArrayList<>();

        @SuppressWarnings("unchecked")
        public <T> Carrier where(TScopedValue<T> key, T value) {
            keys.add((TScopedValue<Object>) key);
            values.add(value);
            return this;
        }

        private void push() {
            for (int i = 0; i < keys.size(); i++) {
                keys.get(i).bindings.push(values.get(i));
            }
        }

        private void pop() {
            for (TScopedValue<Object> k : keys) {
                k.bindings.pop();
            }
        }

        public void run(Runnable op) {
            push();
            try {
                op.run();
            } finally {
                pop();
            }
        }

        public <R> R call(java.util.concurrent.Callable<? extends R> op) throws Exception {
            push();
            try {
                return op.call();
            } finally {
                pop();
            }
        }

        public <R> R get(java.util.function.Supplier<? extends R> op) {
            push();
            try {
                return op.get();
            } finally {
                pop();
            }
        }
    }
}
