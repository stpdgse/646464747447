package com.kamikazeskies.build;

/**
 * The blackbox: a ring buffer of telemetry samples (4 per second, the last 30 seconds) plus running maxima. When a drone
 * carrying a blackbox is lost, the recorder is turned into a {@link Summary} and a compact integer encoding for the item.
 */
public final class FlightRecorder {
    public static final int SAMPLE_INTERVAL_TICKS = 5, CAPACITY = 120, INTS_PER_SAMPLE = 7;

    /** One telemetry sample. Positions in blocks, speed in m/s, voltage in volts, rssi and motor 0..1. */
    public record Sample(int tick, float x, float y, float z, float speed, float volts, float rssi) {}

    public record Summary(double flightSeconds, double maxSpeedKmh, double maxAltitude, double minVolts, double minRssi, double endSpeedKmh) {}

    private final Sample[] ring = new Sample[CAPACITY];
    private int count, head;
    private double maxSpeed, maxAltitude = -1e9, minVolts = 1e9, minRssi = 1.0, lastSpeed;
    private int startTick = -1, lastTick;

    /** Offer a sample; it is stored only every {@link #SAMPLE_INTERVAL_TICKS} ticks. Maxima are tracked every call. */
    public void record(int tick, double x, double y, double z, double speed, double volts, double rssi, double launchY) {
        if (startTick < 0) startTick = tick;
        lastTick = tick;
        maxSpeed = Math.max(maxSpeed, speed);
        maxAltitude = Math.max(maxAltitude, y - launchY);
        minVolts = Math.min(minVolts, volts);
        minRssi = Math.min(minRssi, rssi);
        lastSpeed = speed;
        if ((tick - startTick) % SAMPLE_INTERVAL_TICKS != 0) return;
        ring[head] = new Sample(tick, (float) x, (float) y, (float) z, (float) speed, (float) volts, (float) rssi);
        head = (head + 1) % CAPACITY;
        count = Math.min(CAPACITY, count + 1);
    }

    public int size() { return count; }

    /** Samples oldest first. */
    public Sample[] samples() {
        Sample[] out = new Sample[count];
        for (int i = 0; i < count; i++) out[i] = ring[Math.floorMod(head - count + i, CAPACITY)];
        return out;
    }

    public Summary summary() {
        if (startTick < 0) return new Summary(0, 0, 0, 0, 0, 0);
        return new Summary((lastTick - startTick) / 20.0, maxSpeed * 3.6, Math.max(0, maxAltitude), minVolts, minRssi, lastSpeed * 3.6);
    }

    /** Fixed-point integer encoding of the samples (for a data component): 7 ints per sample. */
    public int[] encodeSamples() {
        Sample[] s = samples();
        int[] out = new int[s.length * INTS_PER_SAMPLE];
        for (int i = 0; i < s.length; i++) {
            int o = i * INTS_PER_SAMPLE;
            out[o] = s[i].tick();
            out[o + 1] = Math.round(s[i].x() * 10);
            out[o + 2] = Math.round(s[i].y() * 10);
            out[o + 3] = Math.round(s[i].z() * 10);
            out[o + 4] = Math.round(s[i].speed() * 100);
            out[o + 5] = Math.round(s[i].volts() * 100);
            out[o + 6] = Math.round(s[i].rssi() * 1000);
        }
        return out;
    }

    public static Sample[] decodeSamples(int[] data) {
        int n = data.length / INTS_PER_SAMPLE;
        Sample[] out = new Sample[n];
        for (int i = 0; i < n; i++) {
            int o = i * INTS_PER_SAMPLE;
            out[i] = new Sample(data[o], data[o + 1] / 10f, data[o + 2] / 10f, data[o + 3] / 10f, data[o + 4] / 100f, data[o + 5] / 100f, data[o + 6] / 1000f);
        }
        return out;
    }
}
