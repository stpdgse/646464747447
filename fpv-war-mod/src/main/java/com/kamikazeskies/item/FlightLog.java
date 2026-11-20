package com.kamikazeskies.item;

import java.util.Arrays;

import com.kamikazeskies.build.FlightRecorder;
import com.mojang.serialization.Codec;
import com.mojang.serialization.codecs.RecordCodecBuilder;

import io.netty.buffer.ByteBuf;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;

/**
 * What a blackbox recovered from a lost drone: why it was lost, a summary, and the last 30 seconds of telemetry
 * (fixed-point integers, see {@link FlightRecorder#encodeSamples()}).
 */
public record FlightLog(int cause, float flightSeconds, float maxSpeedKmh, float maxAltitude, float minVolts, float minRssi, float endSpeedKmh, int[] samples) {
    public static final int CRASH = 0, WARHEAD = 1, LINK_LOST = 2, BATTERY_EMPTY = 3;

    public static FlightLog of(int cause, FlightRecorder r) {
        FlightRecorder.Summary s = r.summary();
        return new FlightLog(cause, (float) s.flightSeconds(), (float) s.maxSpeedKmh(), (float) s.maxAltitude(), (float) s.minVolts(),
                (float) s.minRssi(), (float) s.endSpeedKmh(), r.encodeSamples());
    }

    public static final Codec<FlightLog> CODEC = RecordCodecBuilder.create(i -> i.group(
            Codec.INT.fieldOf("cause").forGetter(FlightLog::cause),
            Codec.FLOAT.fieldOf("flight_seconds").forGetter(FlightLog::flightSeconds),
            Codec.FLOAT.fieldOf("max_speed_kmh").forGetter(FlightLog::maxSpeedKmh),
            Codec.FLOAT.fieldOf("max_altitude").forGetter(FlightLog::maxAltitude),
            Codec.FLOAT.fieldOf("min_volts").forGetter(FlightLog::minVolts),
            Codec.FLOAT.fieldOf("min_rssi").forGetter(FlightLog::minRssi),
            Codec.FLOAT.fieldOf("end_speed_kmh").forGetter(FlightLog::endSpeedKmh),
            Codec.INT.listOf().xmap(l -> l.stream().mapToInt(Integer::intValue).toArray(), a -> Arrays.stream(a).boxed().toList()).fieldOf("samples").forGetter(FlightLog::samples)
    ).apply(i, FlightLog::new));

    public static final StreamCodec<ByteBuf, FlightLog> STREAM_CODEC = StreamCodec.of(
            (buf, v) -> {
                buf.writeInt(v.cause); buf.writeFloat(v.flightSeconds); buf.writeFloat(v.maxSpeedKmh); buf.writeFloat(v.maxAltitude);
                buf.writeFloat(v.minVolts); buf.writeFloat(v.minRssi); buf.writeFloat(v.endSpeedKmh);
                buf.writeInt(v.samples.length);
                for (int x : v.samples) buf.writeInt(x);
            },
            buf -> {
                int cause = buf.readInt(); float a = buf.readFloat(), b = buf.readFloat(), c = buf.readFloat(), d = buf.readFloat(), e = buf.readFloat(), f = buf.readFloat();
                int[] s = new int[Math.min(buf.readInt(), FlightRecorder.CAPACITY * FlightRecorder.INTS_PER_SAMPLE)];
                for (int k = 0; k < s.length; k++) s[k] = buf.readInt();
                return new FlightLog(cause, a, b, c, d, e, f, s);
            });

    @Override public boolean equals(Object o) {
        return o instanceof FlightLog l && cause == l.cause && flightSeconds == l.flightSeconds && maxSpeedKmh == l.maxSpeedKmh
                && maxAltitude == l.maxAltitude && minVolts == l.minVolts && minRssi == l.minRssi && endSpeedKmh == l.endSpeedKmh && Arrays.equals(samples, l.samples);
    }

    @Override public int hashCode() { return 31 * Arrays.hashCode(samples) + Float.hashCode(flightSeconds) + cause; }
}
