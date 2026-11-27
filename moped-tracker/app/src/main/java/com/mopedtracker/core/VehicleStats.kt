package com.mopedtracker.core

import com.mopedtracker.data.SessionEntity

/** Lifetime totals for one vehicle (or for rides with no vehicle). */
data class VehicleStats(
    val rides: Int = 0,
    val distanceM: Double = 0.0,
    val movingMs: Long = 0,
    val topSpeedMps: Float = 0f,
)

object VehicleStatsCalc {
    fun compute(sessions: List<SessionEntity>): Map<Long?, VehicleStats> =
        sessions.groupBy { it.vehicleId }.mapValues { (_, rides) ->
            VehicleStats(
                rides = rides.size,
                distanceM = rides.sumOf { it.distanceM },
                movingMs = rides.sumOf { it.movingMs },
                topSpeedMps = rides.maxOf { it.maxSpeedMps },
            )
        }

    /** Top speed reached as a share of the manufacturer's claim, or null when no claim is known. */
    fun shareOfClaimedTop(maxSpeedMps: Float, claimedTopKmh: Double?): Double? =
        claimedTopKmh?.takeIf { it > 0 }?.let { maxSpeedMps * 3.6 / it }
}
