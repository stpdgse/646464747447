package org.teavm.classlib.java.io;

import org.teavm.classlib.java.lang.TObject;

public class TObjectStreamField extends TObject {
    private final String name;
    private final Class<?> type;

    public TObjectStreamField(String name, Class<?> type) {
        this.name = name;
        this.type = type;
    }

    public String getName() {
        return name;
    }

    public Class<?> getType() {
        return type;
    }
}
