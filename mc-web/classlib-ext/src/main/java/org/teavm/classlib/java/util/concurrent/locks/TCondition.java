package org.teavm.classlib.java.util.concurrent.locks;

import org.teavm.classlib.java.util.concurrent.TTimeUnit;

public interface TCondition {
    void await() throws InterruptedException;
    void awaitUninterruptibly();
    long awaitNanos(long nanos) throws InterruptedException;
    boolean await(long time, TTimeUnit unit) throws InterruptedException;
    void signal();
    void signalAll();
}
