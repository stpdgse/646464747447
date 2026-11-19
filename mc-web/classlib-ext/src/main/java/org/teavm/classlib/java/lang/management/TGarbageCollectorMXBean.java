package org.teavm.classlib.java.lang.management;

public interface TGarbageCollectorMXBean {
    String getName();

    long getCollectionCount();

    long getCollectionTime();
}
