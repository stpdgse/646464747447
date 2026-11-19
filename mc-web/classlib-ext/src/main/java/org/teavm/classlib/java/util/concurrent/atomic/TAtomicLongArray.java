package org.teavm.classlib.java.util.concurrent.atomic;

import org.teavm.classlib.java.lang.TObject;

public class TAtomicLongArray extends TObject {
    private final long[] a;

    public TAtomicLongArray(int length) {
        a = new long[length];
    }

    public TAtomicLongArray(long[] source) {
        a = source.clone();
    }

    public final int length() {
        return a.length;
    }

    public final long get(int i) {
        return a[i];
    }

    public final void set(int i, long v) {
        a[i] = v;
    }

    public final void lazySet(int i, long v) {
        a[i] = v;
    }

    public final long getAndSet(int i, long v) {
        long o = a[i];
        a[i] = v;
        return o;
    }

    public final boolean compareAndSet(int i, long expect, long update) {
        if (a[i] == expect) {
            a[i] = update;
            return true;
        }
        return false;
    }

    public final long getAndIncrement(int i) {
        return a[i]++;
    }

    public final long getAndDecrement(int i) {
        return a[i]--;
    }

    public final long getAndAdd(int i, long d) {
        long o = a[i];
        a[i] += d;
        return o;
    }

    public final long incrementAndGet(int i) {
        return ++a[i];
    }

    public final long decrementAndGet(int i) {
        return --a[i];
    }

    public final long addAndGet(int i, long d) {
        return a[i] += d;
    }

    @Override
    public String toString() {
        return java.util.Arrays.toString(a);
    }
}
