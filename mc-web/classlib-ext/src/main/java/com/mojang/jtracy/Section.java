package com.mojang.jtracy;

public class Section implements AutoCloseable {
    static final Section NONE = new Section();

    @Override
    public void close() {
    }
}
