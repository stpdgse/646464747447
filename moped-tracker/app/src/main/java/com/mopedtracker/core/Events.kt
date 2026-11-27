package com.mopedtracker.core

import com.mopedtracker.data.TrackPointEntity
import kotlin.math.abs

enum class EventType(val label: String) {
    HARD_ACCEL("Hard acceleration"),
    HARD_BRAKE("Hard braking"),
}

data class RideEvent(
    val type: EventType,
    /** Milliseconds since the ride's first point. */
    val offsetMs: Long,
    val lat: Double,
    val lon: Double,
    val accelMps2: Float,
    val fromMps: Float,
    val toMps: Float,
)

/**
 * Finds hard acceleration and braking from the recorded speed. Acceleration is measured over a ~2 second
 * window, which smooths GPS speed noise but still catches a real grab of the brake.
 */
object Events {
    const val HARD_ACCEL_MPS2 = 2.2f   // about 0 -> 50 km/h in 6.3 s: a brisk launch for a moped
    const val HARD_BRAKE_MPS2 = -3.0f  // about 0.3 g
    private const val WINDOW_MS = 2000L
    private const val MAX_WINDOW_MS = 3500L
    private const val MERGE_MS = 6000L
    private const val MIN_BRAKE_START_MPS = 3f

    fun detect(points: List<TrackPointEntity>): List<RideEvent> {
        if (points.size < 3) return emptyList()
        val t0 = points.first().time
        val raw = ArrayList<RideEvent>()
        var j = 0
        for (i in points.indices) {
            if (j < i) j = i
            while (j < points.size && points[j].time - points[i].time < WINDOW_MS) j++
            if (j >= points.size) break
            val dtMs = points[j].time - points[i].time
            if (dtMs > MAX_WINDOW_MS) continue // GPS gap: not a trustworthy measurement
            val a = (points[j].speedMps - points[i].speedMps) / (dtMs / 1000f)
            val type = when {
                a >= HARD_ACCEL_MPS2 -> EventType.HARD_ACCEL
                a <= HARD_BRAKE_MPS2 && points[i].speedMps >= MIN_BRAKE_START_MPS -> EventType.HARD_BRAKE
                else -> null
            } ?: continue
            raw += RideEvent(type, points[i].time - t0, points[i].lat, points[i].lon, a, points[i].speedMps, points[j].speedMps)
        }
        // One event per manoeuvre: keep the strongest reading within each burst of the same type.
        val merged = ArrayList<RideEvent>()
        for (e in raw) {
            val prev = merged.lastOrNull { it.type == e.type }
            if (prev != null && e.offsetMs - prev.offsetMs < MERGE_MS) {
                if (abs(e.accelMps2) > abs(prev.accelMps2)) merged[merged.lastIndexOf(prev)] = e.copy()
            } else {
                merged += e
            }
        }
        return merged.sortedBy { it.offsetMs }
    }
}
