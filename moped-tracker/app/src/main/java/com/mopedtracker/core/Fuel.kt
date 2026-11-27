package com.mopedtracker.core

import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.data.SessionEntity

data class FuelStats(
    val fillUps: Int = 0,
    val totalCost: Double = 0.0,
    val totalLiters: Double = 0.0,
    val avgPricePerLiter: Double = 0.0,
    /** Litres per 100 km, from full-tank to full-tank intervals. Null until two full tanks exist. */
    val consumptionLPer100Km: Double? = null,
    val costPerKm: Double? = null,
    /** Distance covered by the intervals used for the figures above. */
    val trackedDistanceM: Double = 0.0,
)

object FuelCalc {
    /**
     * Consumption uses the standard "full-to-full" method: the litres added at a full fill-up (plus any partial
     * fills since the previous full one) were burnt over the distance driven between those two fill-ups.
     * Distance comes from the recorded rides, so rides you didn't record make consumption look better than it is.
     */
    fun compute(entries: List<FuelEntryEntity>, rides: List<SessionEntity>): FuelStats {
        if (entries.isEmpty()) return FuelStats()
        val sorted = entries.sortedBy { it.time }
        val totalCost = sorted.sumOf { it.totalCost }
        val totalLiters = sorted.sumOf { it.liters }

        var litersUsed = 0.0
        var costUsed = 0.0
        var distUsed = 0.0
        val firstFull = sorted.indexOfFirst { it.fullTank }
        if (firstFull >= 0) {
            var ref = sorted[firstFull]
            var litersSince = 0.0
            var costSince = 0.0
            for (i in firstFull + 1 until sorted.size) {
                val e = sorted[i]
                litersSince += e.liters
                costSince += e.totalCost
                if (e.fullTank) {
                    val dist = rides.filter { it.startTime > ref.time && it.startTime <= e.time }.sumOf { it.distanceM }
                    if (dist > 0) {
                        litersUsed += litersSince
                        costUsed += costSince
                        distUsed += dist
                    }
                    ref = e
                    litersSince = 0.0
                    costSince = 0.0
                }
            }
        }
        val km = distUsed / 1000.0
        return FuelStats(
            fillUps = sorted.size,
            totalCost = totalCost,
            totalLiters = totalLiters,
            avgPricePerLiter = if (totalLiters > 0) totalCost / totalLiters else 0.0,
            consumptionLPer100Km = if (km > 0) litersUsed / km * 100 else null,
            costPerKm = if (km > 0) costUsed / km else null,
            trackedDistanceM = distUsed,
        )
    }

    /** US miles per gallon from litres per 100 km. */
    fun mpg(lPer100Km: Double): Double = 235.214583 / lPer100Km
}
