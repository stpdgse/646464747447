package org.teavm.classlib.java.net;

import org.teavm.classlib.java.lang.TObject;

/** Browsers have no name resolution; only literal addresses and loopback exist. */
public class TInetAddress extends TObject {
    private final String host;
    private final String address;

    protected TInetAddress(String host, String address) {
        this.host = host;
        this.address = address;
    }

    public static TInetAddress getByName(String host) throws TUnknownHostException {
        if (host == null || host.isEmpty() || host.equals("localhost")) {
            return getLoopbackAddress();
        }
        if (host.matches("[0-9.]+") || host.contains(":")) {
            return new TInetAddress(host, host);
        }
        throw new TUnknownHostException(host);
    }

    public static TInetAddress getLocalHost() {
        return getLoopbackAddress();
    }

    public static TInetAddress getLoopbackAddress() {
        return new TInetAddress("localhost", "127.0.0.1");
    }

    public String getHostName() {
        return host;
    }

    public String getHostAddress() {
        return address;
    }

    @Override
    public String toString() {
        return host + "/" + address;
    }

    @Override
    public boolean equals(Object o) {
        return o instanceof TInetAddress && ((TInetAddress) o).address.equals(address);
    }

    @Override
    public int hashCode() {
        return address.hashCode();
    }
}
