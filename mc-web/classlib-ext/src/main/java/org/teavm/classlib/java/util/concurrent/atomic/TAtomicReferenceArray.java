package org.teavm.classlib.java.util.concurrent.atomic;

import org.teavm.classlib.java.lang.TObject;

public class TAtomicReferenceArray<E> extends TObject {
    private final Object[] a;

    public TAtomicReferenceArray(int length) {
        a = new Object[length];
    }

    public TAtomicReferenceArray(E[] source) {
        a = source.clone();
    }

    public final int length() {
        return a.length;
    }

    @SuppressWarnings("unchecked")
    public final E get(int i) {
        return (E) a[i];
    }

    public final void set(int i, E v) {
        a[i] = v;
    }

    public final void lazySet(int i, E v) {
        a[i] = v;
    }

    @SuppressWarnings("unchecked")
    public final E getAndSet(int i, E v) {
        E o = (E) a[i];
        a[i] = v;
        return o;
    }

    public final boolean compareAndSet(int i, E expect, E update) {
        if (a[i] == expect) {
            a[i] = update;
            return true;
        }
        return false;
    }

    @Override
    public String toString() {
        return java.util.Arrays.toString(a);
    }
}
