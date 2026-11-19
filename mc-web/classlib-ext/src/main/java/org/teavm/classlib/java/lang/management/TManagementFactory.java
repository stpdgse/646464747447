package org.teavm.classlib.java.lang.management;

import java.util.Collections;
import java.util.List;
import org.teavm.classlib.java.lang.TObject;

public final class TManagementFactory extends TObject {
    private TManagementFactory() {
    }

    public static TRuntimeMXBean getRuntimeMXBean() {
        return () -> Collections.<String>emptyList();
    }

    public static TMemoryMXBean getMemoryMXBean() {
        long max = 2L * 1024 * 1024 * 1024;
        return new TMemoryMXBean() {
            @Override
            public TMemoryUsage getHeapMemoryUsage() {
                long total = Runtime.getRuntime().totalMemory();
                long free = Runtime.getRuntime().freeMemory();
                return new TMemoryUsage(0, Math.max(0, total - free), total, max);
            }

            @Override
            public TMemoryUsage getNonHeapMemoryUsage() {
                return new TMemoryUsage(0, 0, 0, -1);
            }
        };
    }

    public static TThreadMXBean getThreadMXBean() {
        return new TThreadMXBean() {
            @Override
            public boolean isObjectMonitorUsageSupported() {
                return false;
            }

            @Override
            public boolean isSynchronizerUsageSupported() {
                return false;
            }

            @Override
            public TThreadInfo[] dumpAllThreads(boolean m, boolean s) {
                return new TThreadInfo[] {new TThreadInfo(Thread.currentThread().getName())};
            }
        };
    }

    public static List<TGarbageCollectorMXBean> getGarbageCollectorMXBeans() {
        return Collections.emptyList();
    }
}
