package org.teavm.classlib.java.lang;

import org.teavm.classlib.java.security.TPermission;

public final class TRuntimePermission extends TPermission {
    public TRuntimePermission(String name) {
        super(name);
    }

    @Override
    public boolean implies(TPermission permission) {
        return true;
    }

    @Override
    public String getActions() {
        return "";
    }
}
