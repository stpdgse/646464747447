package org.teavm.classlib.java.net;

public class TInetSocketAddress extends TSocketAddress {
    private final String hostname;
    private final int port;
    private final TInetAddress address;

    public TInetSocketAddress(int port) {
        this("0.0.0.0", port);
    }

    public TInetSocketAddress(String hostname, int port) {
        this.hostname = hostname;
        this.port = port;
        TInetAddress a = null;
        try {
            a = TInetAddress.getByName(hostname);
        } catch (TUnknownHostException e) {
            // unresolved
        }
        this.address = a;
    }

    public TInetSocketAddress(TInetAddress address, int port) {
        this.hostname = address.getHostName();
        this.port = port;
        this.address = address;
    }

    public static TInetSocketAddress createUnresolved(String host, int port) {
        return new TInetSocketAddress(host, port);
    }

    public final int getPort() {
        return port;
    }

    public final TInetAddress getAddress() {
        return address;
    }

    public final String getHostName() {
        return hostname;
    }

    public final String getHostString() {
        return hostname;
    }

    public final boolean isUnresolved() {
        return address == null;
    }

    @Override
    public String toString() {
        return hostname + ":" + port;
    }
}
