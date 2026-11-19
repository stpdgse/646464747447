package org.teavm.classlib.java.lang.management;

public interface TMemoryMXBean {
    TMemoryUsage getHeapMemoryUsage();

    TMemoryUsage getNonHeapMemoryUsage();
}
