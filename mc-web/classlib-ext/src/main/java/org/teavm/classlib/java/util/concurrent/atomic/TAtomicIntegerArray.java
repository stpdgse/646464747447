package org.teavm.classlib.java.util.concurrent.atomic;

import org.teavm.classlib.java.lang.TObject;

public class TAtomicIntegerArray extends TObject {
    private final int[] a;

    public TAtomicIntegerArray(int length) {
        a = new int[length];
    }

    public TAtomicIntegerArray(int[] source) {
        a = source.clone();
    }

    public final int length() {
        return a.length;
    }

    public final int get(int i) {
        return a[i];
    }

    public final void set(int i, int v) {
        a[i] = v;
    }

    public final void lazySet(int i, int v) {
        a[i] = v;
    }

    public final int getAndSet(int i, int v) {
        int o = a[i];
        a[i] = v;
        return o;
    }

    public final boolean compareAndSet(int i, int expect, int update) {
        if (a[i] == expect) {
            a[i] = update;
            return true;
        }
        return false;
    }

    public final int getAndIncrement(int i) {
        return a[i]++;
    }

    public final int getAndDecrement(int i) {
        return a[i]--;
    }

    public final int getAndAdd(int i, int d) {
        int o = a[i];
        a[i] += d;
        return o;
    }

    public final int incrementAndGet(int i) {
        return ++a[i];
    }

    public final int decrementAndGet(int i) {
        return --a[i];
    }

    public final int addAndGet(int i, int d) {
        return a[i] += d;
    }

    @Override
    public String toString() {
        return java.util.Arrays.toString(a);
    }
}
