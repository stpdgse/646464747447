package com.mojang.jtracy;

public class GpuContext {
    public GpuContext setName(String name) {
        return this;
    }

    public void beginZone(int id, String function, String source, String name, int line) {
    }

    public void endZone(int id) {
    }

    public void submitQueryTimestamp(int id, long timestamp) {
    }
}
