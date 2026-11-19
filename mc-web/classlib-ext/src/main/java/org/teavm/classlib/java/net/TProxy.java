package org.teavm.classlib.java.net;

import org.teavm.classlib.java.lang.TObject;

public class TProxy extends TObject {
    public enum Type {
        DIRECT, HTTP, SOCKS
    }

    public static final TProxy NO_PROXY = new TProxy();

    private final Type type;
    private final TSocketAddress address;

    private TProxy() {
        this.type = Type.DIRECT;
        this.address = null;
    }

    public TProxy(Type type, TSocketAddress address) {
        this.type = type;
        this.address = address;
    }

    public Type type() {
        return type;
    }

    public TSocketAddress address() {
        return address;
    }
}
