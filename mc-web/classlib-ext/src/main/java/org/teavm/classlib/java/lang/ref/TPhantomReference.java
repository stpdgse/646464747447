package org.teavm.classlib.java.lang.ref;

public class TPhantomReference<T> extends TReference<T> {
    public TPhantomReference(T referent, TReferenceQueue<? super T> queue) {
    }

    @Override
    public T get() {
        return null;
    }

    @Override
    public boolean enqueue() {
        return false;
    }
}
