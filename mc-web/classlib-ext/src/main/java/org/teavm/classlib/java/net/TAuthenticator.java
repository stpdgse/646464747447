package org.teavm.classlib.java.net;

import org.teavm.classlib.java.lang.TObject;

public abstract class TAuthenticator extends TObject {
    private static TAuthenticator theDefault;

    public static synchronized void setDefault(TAuthenticator a) {
        theDefault = a;
    }

    protected TPasswordAuthentication getPasswordAuthentication() {
        return null;
    }
}
