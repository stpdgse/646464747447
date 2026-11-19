package org.teavm.classlib.java.util.concurrent.locks;

import java.util.HashSet;
import java.util.Set;
import org.teavm.classlib.java.lang.TObject;

public class TLockSupport extends TObject {
    private static final Object MONITOR = new Object();
    private static final Set<Thread> PERMITS = new HashSet<>();

    private TLockSupport() {
    }

    public static void park() {
        park0(0);
    }

    public static void park(Object blocker) {
        park0(0);
    }

    public static void parkNanos(long nanos) {
        if (nanos > 0) {
            park0(nanos);
        }
    }

    public static void parkNanos(Object blocker, long nanos) {
        parkNanos(nanos);
    }

    private static void park0(long nanos) {
        Thread current = Thread.currentThread();
        long deadline = nanos > 0 ? System.nanoTime() + nanos : 0;
        synchronized (MONITOR) {
            while (!PERMITS.remove(current)) {
                try {
                    if (deadline == 0) {
                        MONITOR.wait();
                    } else {
                        long left = (deadline - System.nanoTime()) / 1_000_000;
                        if (left <= 0) {
                            return;
                        }
                        MONITOR.wait(left);
                    }
                } catch (InterruptedException e) {
                    current.interrupt();
                    return;
                }
            }
        }
    }

    public static void unpark(Thread thread) {
        if (thread == null) {
            return;
        }
        synchronized (MONITOR) {
            PERMITS.add(thread);
            MONITOR.notifyAll();
        }
    }
}
