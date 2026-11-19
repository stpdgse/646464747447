package org.teavm.classlib.java.lang.management;

public interface TThreadMXBean {
    boolean isObjectMonitorUsageSupported();

    boolean isSynchronizerUsageSupported();

    TThreadInfo[] dumpAllThreads(boolean lockedMonitors, boolean lockedSynchronizers);
}
