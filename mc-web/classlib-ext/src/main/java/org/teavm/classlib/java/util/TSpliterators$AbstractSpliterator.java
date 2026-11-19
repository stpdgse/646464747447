package org.teavm.classlib.java.util;

import org.teavm.classlib.java.lang.TObject;

/**
 * java.util.Spliterators.AbstractSpliterator. TeaVM's TSpliterators lacks this nested class;
 * the mapped name {@code TSpliterators$AbstractSpliterator} lets us supply it from outside.
 */
public abstract class TSpliterators$AbstractSpliterator<T> extends TObject implements TSpliterator<T> {
    private long est;
    private final int characteristics;

    protected TSpliterators$AbstractSpliterator(long est, int additionalCharacteristics) {
        this.est = est;
        this.characteristics = (additionalCharacteristics & TSpliterator.SIZED) != 0
                ? additionalCharacteristics | TSpliterator.SUBSIZED : additionalCharacteristics;
    }

    @Override
    public TSpliterator<T> trySplit() {
        return null;
    }

    @Override
    public long estimateSize() {
        return est;
    }

    @Override
    public int characteristics() {
        return characteristics;
    }
}
