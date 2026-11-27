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
        /** Position worse than this is unusable, but the Doppler speed may still be good (phone in a pocket). */
        val maxDegradedAccuracyM: Float = 60f,
        val degradedSpeedAccuracyMps: Float = 1.5f,
    )

    /**
     * [positionTrusted] is false when only the Doppler speed was usable: the stats were updated,
     * but the fix's coordinates must not be drawn on the route.
     */
    data class Result(val accepted: Boolean, val speedMps: Float, val positionTrusted: Boolean = true)

    private var last: Fix? = null
    private var lastSpeed = 0f
    private var rejectStreak = 0

    /** Time of the most recent fix of any kind (trusted or speed-only). */
    private var lastTimeMs: Long? = null
    /** Distance already added from speed integration since the last trusted position. */
    private var integratedSinceTrusted = 0.0

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
        lastTimeMs = null
        integratedSinceTrusted = 0.0
        rejectStreak = 0
    }

    fun add(fix: Fix): Result {
        if (fix.accuracyM > cfg.maxAccuracyM) return addSpeedOnly(fix)

        val prev = last
        var dtPos = 0.0 // time since the last trusted position (for deriving speed)
        var segment = 0.0
        if (prev != null) {
            dtPos = (fix.timeMs - prev.timeMs) / 1000.0
            if (dtPos <= 0.0) return Result(false, 0f) // duplicate or out-of-order fix
            segment = Geo.distanceM(prev.lat, prev.lon, fix.lat, fix.lon)
        }
        // Time since the latest fix of any kind (moving time was already credited up to there).
        val refTime = lastTimeMs ?: prev?.timeMs
        val dt = if (refTime != null) (fix.timeMs - refTime) / 1000.0 else 0.0
        if (refTime != null && dt <= 0.0) return Result(false, 0f)

        val speed = fix.speedMps ?: if (prev != null) (segment / dtPos).toFloat() else 0f
        if (speed.isNaN() || speed < 0f || speed > cfg.maxPlausibleSpeedMps) return reject()

        if (refTime != null) {
            val accel = abs(speed - lastSpeed) / dt
            if (accel > cfg.maxAccelMps2 && rejectStreak < cfg.resyncAfterRejects) return reject()
        }
        rejectStreak = 0

        val moving = speed >= cfg.movingThresholdMps
        if (refTime != null && moving) {
            movingMs += (dt * 1000).toLong().coerceAtMost(cfg.maxGapMs)
        }
        if (prev != null && moving) {
            // If speed integration already covered part of this stretch, only add what the straight
            // line exceeds it by; the integrated path (which follows the curves) is the better estimate.
            val extra = if (integratedSinceTrusted > 0.0) (segment - integratedSinceTrusted).coerceAtLeast(0.0)
            else if (segment >= cfg.minSegmentM) segment else 0.0
            distanceM += extra
        }
        integratedSinceTrusted = 0.0
        lastTimeMs = fix.timeMs

        val speedTrusted = fix.speedAccuracyMps == null || fix.speedAccuracyMps <= cfg.maxSpeedAccuracyForMaxMps
        if (speedTrusted && speed > maxSpeed) maxSpeed = speed

        updateElevation(fix.altitudeM)

        accuracySum += fix.accuracyM
        count++
        last = fix
        lastSpeed = speed
        return Result(true, speed)
    }

    /**
     * Fix with poor position accuracy (typical with the phone in a pocket). The position is useless but
     * the Doppler speed is measured independently of it, so a fix whose speed accuracy is good still
     * counts: its speed feeds the max/moving time and distance advances by speed x time.
     */
    private fun addSpeedOnly(fix: Fix): Result {
        val ref = lastTimeMs ?: return Result(false, 0f) // need a starting point
        val speed = fix.speedMps
        val speedAcc = fix.speedAccuracyMps
        if (speed == null || speedAcc == null || speedAcc > cfg.degradedSpeedAccuracyMps ||
            fix.accuracyM > cfg.maxDegradedAccuracyM
        ) return Result(false, 0f)

        val dt = (fix.timeMs - ref) / 1000.0
        if (dt <= 0.0) return Result(false, 0f)
        if (speed.isNaN() || speed < 0f || speed > cfg.maxPlausibleSpeedMps) return Result(false, 0f)
        if (abs(speed - lastSpeed) / dt > cfg.maxAccelMps2 && rejectStreak < cfg.resyncAfterRejects) {
            rejectStreak++
            return Result(false, 0f)
        }
        rejectStreak = 0

        if (speed >= cfg.movingThresholdMps) {
            val capped = dt.coerceAtMost(cfg.maxGapMs / 1000.0)
            movingMs += (capped * 1000).toLong()
            val d = (lastSpeed + speed) / 2.0 * capped // trapezoid between the two speed samples
            distanceM += d
            integratedSinceTrusted += d
        }
        if (speed > maxSpeed) maxSpeed = speed
        lastTimeMs = fix.timeMs
        lastSpeed = speed
        return Result(true, speed, positionTrusted = false)
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
