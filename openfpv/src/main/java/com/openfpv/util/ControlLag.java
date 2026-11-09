package com.openfpv.util;

import java.util.ArrayDeque;
import java.util.Random;

/**
 * Models a radio control link: as the signal weakens, stick inputs arrive later and some are lost.
 * Pure Java (unit tested). The client pushes its sticks every tick and sends whatever poll() returns.
 */
public final class ControlLag {
    /** The four control sticks, each in [-1, 1]. */
    public static final class Sticks {
        public final float pitch, roll, yaw, throttle;

        public Sticks(float pitch, float roll, float yaw, float throttle) {
            this.pitch = pitch;
            this.roll = roll;
            this.yaw = yaw;
            this.throttle = throttle;
        }
    }

    private static final class Entry {
        final long due;
        final Sticks sticks;

        Entry(long due, Sticks sticks) {
            this.due = due;
            this.sticks = sticks;
        }
    }

    private final ArrayDeque<Entry> queue = new ArrayDeque<>();
    private final Random rng;
    private long lastDue = Long.MIN_VALUE;

    public ControlLag(long seed) {
        this.rng = new Random(seed);
    }

    /** Delay in ticks for the given signal quality (1 = perfect, 0 = at the range limit). */
    public static int lagTicks(float quality, int maxLag) {
        return Math.round((1f - CameraMath.clamp(quality, 0f, 1f)) * maxLag);
    }

    /** Probability that a packet is lost for the given signal quality. */
    public static double lossProbability(float quality, double maxLoss) {
        return (1.0 - CameraMath.clamp(quality, 0f, 1f)) * maxLoss;
    }

    /** Queues this tick's sticks; returns false if the "packet" was lost. */
    public boolean push(long tick, Sticks sticks, float quality, int maxLag, double maxLoss) {
        if (maxLoss > 0 && rng.nextDouble() < lossProbability(quality, maxLoss)) return false;
        long due = tick + lagTicks(quality, maxLag);
        if (due < lastDue) due = lastDue; // keep packets in order
        lastDue = due;
        queue.addLast(new Entry(due, sticks));
        return true;
    }

    /** The newest sticks whose delivery time has come, or null if nothing arrives this tick. */
    public Sticks poll(long tick) {
        Sticks latest = null;
        while (!queue.isEmpty() && queue.peekFirst().due <= tick) {
            latest = queue.pollFirst().sticks;
        }
        return latest;
    }

    public void clear() {
        queue.clear();
        lastDue = Long.MIN_VALUE;
    }
}
