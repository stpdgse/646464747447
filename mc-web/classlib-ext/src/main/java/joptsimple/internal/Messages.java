package joptsimple.internal;

import java.util.Locale;

/** jopt-simple message lookup without resource bundles: returns the key and its arguments. */
public class Messages {
    private Messages() {
        throw new UnsupportedOperationException();
    }

    public static String message(Locale locale, String bundleName, Class<?> type, String key, Object... args) {
        StringBuilder sb = new StringBuilder(type.getSimpleName()).append('.').append(key);
        for (Object a : args) {
            sb.append(' ').append(a);
        }
        return sb.toString();
    }
}
