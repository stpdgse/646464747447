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
     *
     * Each vehicle is calculated separately (a fill-up for one vehicle says nothing about another's mileage) and
     * the results are then added up, so "all vehicles" is the true overall figure.
     */
    fun compute(entries: List<FuelEntryEntity>, rides: List<SessionEntity>): FuelStats {
        if (entries.isEmpty()) return FuelStats()
        val totalCost = entries.sumOf { it.totalCost }
        val totalLiters = entries.sumOf { it.liters }

        var litersUsed = 0.0
        var costUsed = 0.0
        var distUsed = 0.0
        for ((vehicleId, group) in entries.groupBy { it.vehicleId }) {
            val (l, c, d) = fullToFull(group.sortedBy { it.time }, rides.filter { it.vehicleId == vehicleId })
            litersUsed += l
            costUsed += c
            distUsed += d
        }
        val km = distUsed / 1000.0
        return FuelStats(
            fillUps = entries.size,
            totalCost = totalCost,
            totalLiters = totalLiters,
            avgPricePerLiter = if (totalLiters > 0) totalCost / totalLiters else 0.0,
            consumptionLPer100Km = if (km > 0) litersUsed / km * 100 else null,
            costPerKm = if (km > 0) costUsed / km else null,
            trackedDistanceM = distUsed,
        )
    }

    /** (litres, cost, metres) covered by complete full-tank intervals of one vehicle's fill-ups. */
    private fun fullToFull(sorted: List<FuelEntryEntity>, rides: List<SessionEntity>): Triple<Double, Double, Double> {
        var litersUsed = 0.0
        var costUsed = 0.0
        var distUsed = 0.0
        val firstFull = sorted.indexOfFirst { it.fullTank }
        if (firstFull < 0) return Triple(0.0, 0.0, 0.0)
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
        return Triple(litersUsed, costUsed, distUsed)
    }

    /** US miles per gallon from litres per 100 km. */
    fun mpg(lPer100Km: Double): Double = 235.214583 / lPer100Km
}
