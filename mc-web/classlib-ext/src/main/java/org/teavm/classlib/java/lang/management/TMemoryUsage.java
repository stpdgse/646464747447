package org.teavm.classlib.java.lang.management;

import org.teavm.classlib.java.lang.TObject;

public class TMemoryUsage extends TObject {
    private final long init;
    private final long used;
    private final long committed;
    private final long max;

    public TMemoryUsage(long init, long used, long committed, long max) {
        this.init = init;
        this.used = used;
        this.committed = committed;
        this.max = max;
    }

    public long getInit() {
        return init;
    }

    public long getUsed() {
        return used;
    }

    public long getCommitted() {
        return committed;
    }

    public long getMax() {
        return max;
    }
}
