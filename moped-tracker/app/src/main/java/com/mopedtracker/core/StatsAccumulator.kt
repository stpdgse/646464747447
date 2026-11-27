package com.mopedtracker.core

import kotlin.math.abs

/** A single raw GPS fix, independent of the Android framework so it can be unit-tested. */
data class Fix(
    val timeMs: Long,
    val lat: Double,
    val lon: Double,
    val altitudeM: Double? = null,
    /** Doppler speed from the GPS chip in m/s, or null when the provider has none. */
    val speedMps: Float? = null,
    val accuracyM: Float = 5f,
    val speedAccuracyMps: Float? = null,
    val bearingDeg: Float? = null,
)

data class RideStats(
    val distanceM: Double = 0.0,
    val elapsedMs: Long = 0,
    val movingMs: Long = 0,
    val stoppedMs: Long = 0,
    val maxSpeedMps: Float = 0f,
    /** Distance divided by total elapsed time (includes stops). */
    val avgSpeedMps: Float = 0f,
    /** Distance divided by moving time only. */
    val avgMovingSpeedMps: Float = 0f,
    val elevationGainM: Double = 0.0,
    val elevationLossM: Double = 0.0,
    val maxAltitudeM: Double? = null,
    val minAltitudeM: Double? = null,
    val avgAccuracyM: Float = 0f,
    val pointCount: Int = 0,
)

/**
 * Turns a stream of noisy GPS fixes into trustworthy ride statistics.
 *
 * Accuracy strategy:
 *  - speed comes from the GPS Doppler measurement (far better than differentiating positions);
 *  - fixes with poor horizontal accuracy are dropped;
 *  - physically impossible speed jumps are dropped (multipath spikes next to buildings, tunnels);
 *  - the max speed additionally requires a good reported speed accuracy;
 *  - distance only accumulates while moving, so GPS drift at a red light adds nothing;
 *  - elevation uses a hysteresis threshold so altitude noise doesn't inflate the climb.
 */
class StatsAccumulator(private val cfg: Config = Config()) {

    data class Config(
        val maxAccuracyM: Float = 25f,
        val movingThresholdMps: Float = 1.0f,
        val maxPlausibleSpeedMps: Float = 70f,
        val maxAccelMps2: Float = 9f,
        val maxSpeedAccuracyForMaxMps: Float = 2f,
        val minSegmentM: Double = 1.0,
        val elevationThresholdM: Double = 3.0,
        val maxGapMs: Long = 10_000,
        val resyncAfterRejects: Int = 3,
    )

    data class Result(val accepted: Boolean, val speedMps: Float)

    private var last: Fix? = null
    private var lastSpeed = 0f
    private var rejectStreak = 0

    private var distanceM = 0.0
    private var movingMs = 0L
    private var maxSpeed = 0f
    private var accuracySum = 0.0
    private var count = 0

    private var refAlt: Double? = null
    private var gain = 0.0
    private var loss = 0.0
    private var maxAlt: Double? = null
    private var minAlt: Double? = null

    /** Forget the previous fix (use after a pause so the gap isn't counted as travel). */
    fun breakLink() {
        last = null
        rejectStreak = 0
    }

    fun add(fix: Fix): Result {
        if (fix.accuracyM > cfg.maxAccuracyM) return Result(false, 0f)

        val prev = last
        var dt = 0.0
        var segment = 0.0
        if (prev != null) {
            dt = (fix.timeMs - prev.timeMs) / 1000.0
            if (dt <= 0.0) return Result(false, 0f) // duplicate or out-of-order fix
            segment = Geo.distanceM(prev.lat, prev.lon, fix.lat, fix.lon)
        }

        val speed = fix.speedMps ?: if (prev != null) (segment / dt).toFloat() else 0f
        if (speed.isNaN() || speed < 0f || speed > cfg.maxPlausibleSpeedMps) return reject()

        if (prev != null) {
            val accel = abs(speed - lastSpeed) / dt
            if (accel > cfg.maxAccelMps2 && rejectStreak < cfg.resyncAfterRejects) return reject()
        }
        rejectStreak = 0

        val moving = speed >= cfg.movingThresholdMps
        if (prev != null) {
            val dtMs = (dt * 1000).toLong().coerceAtMost(cfg.maxGapMs)
            if (moving) {
                movingMs += dtMs
                if (segment >= cfg.minSegmentM) distanceM += segment
            }
        }

        val speedTrusted = fix.speedAccuracyMps == null || fix.speedAccuracyMps <= cfg.maxSpeedAccuracyForMaxMps
        if (speedTrusted && speed > maxSpeed) maxSpeed = speed

        updateElevation(fix.altitudeM)

        accuracySum += fix.accuracyM
        count++
        last = fix
        lastSpeed = speed
        return Result(true, speed)
    }

    fun snapshot(elapsedMs: Long): RideStats {
        val elapsedS = elapsedMs / 1000.0
        val movingS = movingMs / 1000.0
        return RideStats(
            distanceM = distanceM,
            elapsedMs = elapsedMs,
            movingMs = movingMs,
            stoppedMs = (elapsedMs - movingMs).coerceAtLeast(0),
            maxSpeedMps = maxSpeed,
            avgSpeedMps = if (elapsedS > 0) (distanceM / elapsedS).toFloat() else 0f,
            avgMovingSpeedMps = if (movingS > 0) (distanceM / movingS).toFloat() else 0f,
            elevationGainM = gain,
            elevationLossM = loss,
            maxAltitudeM = maxAlt,
            minAltitudeM = minAlt,
            avgAccuracyM = if (count > 0) (accuracySum / count).toFloat() else 0f,
            pointCount = count,
        )
    }

    private fun reject(): Result {
        rejectStreak++
        return Result(false, 0f)
    }

    private fun updateElevation(alt: Double?) {
        if (alt == null) return
        maxAlt = maxOf(maxAlt ?: alt, alt)
        minAlt = minOf(minAlt ?: alt, alt)
        val ref = refAlt
        if (ref == null) {
            refAlt = alt
            return
        }
        val diff = alt - ref
        if (diff >= cfg.elevationThresholdM) {
            gain += diff
            refAlt = alt
        } else if (-diff >= cfg.elevationThresholdM) {
            loss += -diff
            refAlt = alt
        }
    }
}
