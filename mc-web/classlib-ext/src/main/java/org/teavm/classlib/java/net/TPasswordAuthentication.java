package org.teavm.classlib.java.net;

import org.teavm.classlib.java.lang.TObject;

public final class TPasswordAuthentication extends TObject {
    private final String user;
    private final char[] password;

    public TPasswordAuthentication(String user, char[] password) {
        this.user = user;
        this.password = password.clone();
    }

    public String getUserName() {
        return user;
    }

    public char[] getPassword() {
        return password;
    }
}
