package com.mojang.jtracy;

/** No-op profiler zone (the Tracy profiler is not available in the browser). */
public class Zone implements AutoCloseable {
    static final Zone NONE = new Zone();

    public Zone addText(String text) {
        return this;
    }

    public Zone setColor(int color) {
        return this;
    }

    public Zone addValue(long value) {
        return this;
    }

    @Override
    public void close() {
    }
}
