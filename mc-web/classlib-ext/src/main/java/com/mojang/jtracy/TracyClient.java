package com.mojang.jtracy;

import java.nio.ByteBuffer;
import java.util.function.Supplier;

/** The profiler is permanently unavailable in the browser; every call is a cheap no-op. */
public class TracyClient {
    public static boolean isAvailable() {
        return false;
    }

    public static void load() {
    }

    public static void markFrame() {
    }

    public static void frameImage(ByteBuffer image, int width, int height, int offset, boolean flip) {
    }

    public static Zone beginZone(String name, boolean active) {
        return Zone.NONE;
    }

    public static Zone beginZone(String function, String file, String name, int line) {
        return Zone.NONE;
    }

    public static void setThreadName(String name, int priority) {
    }

    public static Plot createPlot(String name) {
        return new Plot();
    }

    public static DiscontinuousFrame createDiscontinuousFrame(String name) {
        return new DiscontinuousFrame();
    }

    public static ContinuousFrame createContinuousFrame(String name) {
        return new ContinuousFrame();
    }

    public static MemoryPool createMemoryPool(String name) {
        return new MemoryPool();
    }

    public static void reportAppInfo(String info) {
    }

    public static void message(String message) {
    }

    public static void message(String message, int color) {
    }

    public static void message(Supplier<String> message) {
    }

    public static void message(Supplier<String> message, int color) {
    }

    public static GpuContext createGpuContext(GpuApi api, long timestamp, float period) {
        return new GpuContext();
    }

    public static SectionCategory createSectionCategory(String name) {
        return new SectionCategory();
    }
}
