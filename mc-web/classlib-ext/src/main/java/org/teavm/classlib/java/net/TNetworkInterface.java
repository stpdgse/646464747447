package org.teavm.classlib.java.net;

import java.util.Collections;
import java.util.Enumeration;
import java.util.List;
import org.teavm.classlib.java.lang.TObject;

/** No network adapters are visible to a web page. */
public final class TNetworkInterface extends TObject {
    private TNetworkInterface() {
    }

    public static Enumeration<TNetworkInterface> getNetworkInterfaces() {
        return Collections.emptyEnumeration();
    }

    public static TNetworkInterface getByName(String name) {
        return null;
    }

    public static TNetworkInterface getByIndex(int index) {
        return null;
    }

    public static TNetworkInterface getByInetAddress(TInetAddress address) {
        return null;
    }

    public String getName() {
        return "";
    }

    public String getDisplayName() {
        return "";
    }

    public int getIndex() {
        return 0;
    }

    public int getMTU() {
        return 0;
    }

    public boolean isUp() {
        return false;
    }

    public boolean isLoopback() {
        return false;
    }

    public boolean isVirtual() {
        return false;
    }

    public byte[] getHardwareAddress() {
        return null;
    }

    public Enumeration<TInetAddress> getInetAddresses() {
        return Collections.emptyEnumeration();
    }

    public List<Object> getInterfaceAddresses() {
        return Collections.emptyList();
    }
}
