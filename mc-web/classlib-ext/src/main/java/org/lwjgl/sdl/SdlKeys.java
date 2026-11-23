package org.lwjgl.sdl;

/** SDL scancode / keycode tables for a US layout; the page supplies layout-aware keycodes for real events. */
final class SdlKeys {
    static final int EXTENDED = 0x40000000;

    private SdlKeys() {
    }

    private static final String[] NAMES = new String[512];

    static {
        String[] fkeys = new String[12];
        for (int i = 0; i < 12; i++) {
            NAMES[58 + i] = "F" + (i + 1);
        }
        NAMES[40] = "Return";
        NAMES[41] = "Escape";
        NAMES[42] = "Backspace";
        NAMES[43] = "Tab";
        NAMES[44] = "Space";
        NAMES[57] = "CapsLock";
        NAMES[70] = "PrintScreen";
        NAMES[71] = "ScrollLock";
        NAMES[72] = "Pause";
        NAMES[73] = "Insert";
        NAMES[74] = "Home";
        NAMES[75] = "PageUp";
        NAMES[76] = "Delete";
        NAMES[77] = "End";
        NAMES[78] = "PageDown";
        NAMES[79] = "Right";
        NAMES[80] = "Left";
        NAMES[81] = "Down";
        NAMES[82] = "Up";
        NAMES[83] = "Numlock";
        NAMES[84] = "Keypad /";
        NAMES[85] = "Keypad *";
        NAMES[86] = "Keypad -";
        NAMES[87] = "Keypad +";
        NAMES[88] = "Keypad Enter";
        for (int i = 0; i < 9; i++) {
            NAMES[89 + i] = "Keypad " + (i + 1);
        }
        NAMES[98] = "Keypad 0";
        NAMES[99] = "Keypad .";
        NAMES[224] = "Left Ctrl";
        NAMES[225] = "Left Shift";
        NAMES[226] = "Left Alt";
        NAMES[227] = "Left GUI";
        NAMES[228] = "Right Ctrl";
        NAMES[229] = "Right Shift";
        NAMES[230] = "Right Alt";
        NAMES[231] = "Right GUI";
    }

    private static final String PUNCT = "-=[]\\#;'`,./";

    /** Keycode for a scancode on a US keyboard (unshifted). */
    static int keycode(int sc, boolean shift) {
        int k;
        if (sc >= 4 && sc <= 29) {
            k = 'a' + (sc - 4);
            return shift ? Character.toUpperCase(k) : k;
        }
        if (sc >= 30 && sc <= 38) {
            return shift ? ")!@#$%^&*(".charAt(sc - 29) : '1' + (sc - 30);
        }
        if (sc == 39) {
            return shift ? ')' : '0';
        }
        switch (sc) {
            case 40: return 13;
            case 41: return 27;
            case 42: return 8;
            case 43: return 9;
            case 44: return 32;
            case 45: return shift ? '_' : '-';
            case 46: return shift ? '+' : '=';
            case 47: return shift ? '{' : '[';
            case 48: return shift ? '}' : ']';
            case 49: return shift ? '|' : '\\';
            case 51: return shift ? ':' : ';';
            case 52: return shift ? '"' : '\'';
            case 53: return shift ? '~' : '`';
            case 54: return shift ? '<' : ',';
            case 55: return shift ? '>' : '.';
            case 56: return shift ? '?' : '/';
            case 76: return 127;
            default: return sc | EXTENDED;
        }
    }

    static String name(int keycode) {
        if ((keycode & EXTENDED) != 0) {
            int sc = keycode & ~EXTENDED;
            return sc >= 0 && sc < NAMES.length && NAMES[sc] != null ? NAMES[sc] : "";
        }
        switch (keycode) {
            case 13: return "Return";
            case 27: return "Escape";
            case 8: return "Backspace";
            case 9: return "Tab";
            case 32: return "Space";
            case 127: return "Delete";
            default: return keycode > 32 ? new String(Character.toChars(Character.toUpperCase(keycode))) : "";
        }
    }
}
