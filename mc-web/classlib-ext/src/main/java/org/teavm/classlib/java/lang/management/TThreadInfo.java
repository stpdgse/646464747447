package org.teavm.classlib.java.lang.management;

import org.teavm.classlib.java.lang.TObject;

public class TThreadInfo extends TObject {
    private final String name;

    TThreadInfo(String name) {
        this.name = name;
    }

    public String getThreadName() {
        return name;
    }

    @Override
    public String toString() {
        return "\"" + name + "\"";
    }
}
