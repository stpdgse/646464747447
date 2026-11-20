package com.kamikazeskies.input;

/** Flight assist presets, selectable in flight. The sim accepts any value in [0, 1]; these are the quick-switch stops. */
public enum AssistMode {
    ANGLE(1.0),   // self-leveling, easiest
    MIXED(0.5),   // partial leveling, forgiving but lively
    ACRO(0.0);    // full manual rates

    public final double level;

    AssistMode(double level) { this.level = level; }

    public AssistMode next() { return values()[(ordinal() + 1) % values().length]; }
}
