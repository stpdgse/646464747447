package com.mopedtracker

import com.mopedtracker.core.EventType
import com.mopedtracker.core.Events
import com.mopedtracker.core.FuelCalc
import com.mopedtracker.core.RecordKind
import com.mopedtracker.core.Records
import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FeatureLogicTest {

    private val day = 86_400_000L

    private fun ride(id: Long, start: Long, km: Double, maxKmh: Float = 40f, avgKmh: Float = 30f, movingMin: Int = 20, climb: Double = 0.0) =
        SessionEntity(
            id = id, name = "r$id", startTime = start, endTime = start + 1, distanceM = km * 1000,
            elapsedMs = movingMin * 60_000L, movingMs = movingMin * 60_000L,
            maxSpeedMps = maxKmh / 3.6f, avgMovingSpeedMps = avgKmh / 3.6f, elevationGainM = climb,
        )

    private fun fill(id: Long, time: Long, liters: Double, cost: Double, full: Boolean = true) =
        FuelEntryEntity(id, time, liters, cost, full)

    // region fuel

    @Test fun fullToFullConsumptionUsesDistanceBetweenFillUps() {
        val fills = listOf(fill(1, 0, 4.0, 6.0), fill(2, 10 * day, 3.0, 4.8), fill(3, 20 * day, 2.5, 4.0))
        val rides = listOf(ride(1, 2 * day, 60.0), ride(2, 5 * day, 40.0), ride(3, 12 * day, 50.0))
        val st = FuelCalc.compute(fills, rides)

        // 3.0 L over the first 100 km, then 2.5 L over the next 50 km  => 5.5 L / 150 km
        assertEquals(5.5 / 150 * 100, st.consumptionLPer100Km!!, 1e-9)
        assertEquals((4.8 + 4.0) / 150, st.costPerKm!!, 1e-9)
        assertEquals(14.8, st.totalCost, 1e-9)
        assertEquals(9.5, st.totalLiters, 1e-9)
        assertEquals(14.8 / 9.5, st.avgPricePerLiter, 1e-9)
        assertEquals(150_000.0, st.trackedDistanceM, 1e-6)
    }

    @Test fun partialFillsAreAddedToTheNextFullFill() {
        val fills = listOf(fill(1, 0, 4.0, 6.0), fill(2, 5 * day, 1.0, 1.6, full = false), fill(3, 10 * day, 2.0, 3.2))
        val st = FuelCalc.compute(fills, listOf(ride(1, 3 * day, 100.0)))
        assertEquals(3.0, st.consumptionLPer100Km!!, 1e-9) // (1.0 + 2.0) L / 100 km
    }

    @Test fun noConsumptionUntilTwoFullTanks() {
        assertNull(FuelCalc.compute(listOf(fill(1, 0, 4.0, 6.0)), listOf(ride(1, day, 50.0))).consumptionLPer100Km)
        assertNull(FuelCalc.compute(emptyList(), emptyList()).consumptionLPer100Km)
    }

    @Test fun ridesOutsideTheIntervalAreNotCounted() {
        val fills = listOf(fill(1, 10 * day, 4.0, 6.0), fill(2, 20 * day, 3.0, 4.5))
        val rides = listOf(ride(1, 5 * day, 999.0), ride(2, 15 * day, 100.0), ride(3, 25 * day, 999.0))
        assertEquals(3.0, FuelCalc.compute(fills, rides).consumptionLPer100Km!!, 1e-9)
    }

    @Test fun mpgConversion() = assertEquals(78.4, FuelCalc.mpg(3.0), 0.1)

    // endregion

    // region records

    @Test fun recordsPickTheBestRidePerCategory() {
        val rides = listOf(
            ride(1, 0, 10.0, maxKmh = 55f, avgKmh = 28f, movingMin = 20, climb = 10.0),
            ride(2, day, 30.0, maxKmh = 48f, avgKmh = 35f, movingMin = 50, climb = 80.0),
            ride(3, 2 * day, 3.0, maxKmh = 70f, avgKmh = 60f, movingMin = 0, climb = 5.0), // too short for "fastest average"
        )
        val r = Records.compute(rides).associateBy { it.kind }
        assertEquals(3L, r[RecordKind.TOP_SPEED]!!.session.id)
        assertEquals(2L, r[RecordKind.LONGEST_DISTANCE]!!.session.id)
        assertEquals(2L, r[RecordKind.LONGEST_DURATION]!!.session.id)
        assertEquals(2L, r[RecordKind.FASTEST_AVERAGE]!!.session.id)
        assertEquals(2L, r[RecordKind.MOST_CLIMB]!!.session.id)
    }

    @Test fun noRecordsWithoutRides() = assertTrue(Records.compute(emptyList()).isEmpty())

    // endregion

    // region events

    /** 1 Hz points; speeds in m/s, heading north. */
    private fun track(speeds: List<Float>) = speeds.mapIndexed { i, v ->
        TrackPointEntity(
            sessionId = 1, time = i * 1000L, lat = 48.0 + i * 1e-4, lon = 11.0, altitude = null,
            speedMps = v, accuracyM = 4f, speedAccuracyMps = 0.4f, bearing = null,
        )
    }

    @Test fun hardBrakingIsDetectedOnce() {
        val speeds = List(10) { 12f } + listOf(8f, 4f, 1f, 0f, 0f, 0f) + List(5) { 0f }
        val ev = Events.detect(track(speeds))
        assertEquals(1, ev.size)
        assertEquals(EventType.HARD_BRAKE, ev[0].type)
        assertTrue(ev[0].accelMps2 <= Events.HARD_BRAKE_MPS2)
    }

    @Test fun gentleSlowingDownIsNotAnEvent() {
        val speeds = List(30) { 12f - it * 0.3f } // -0.3 m/s^2
        assertTrue(Events.detect(track(speeds)).isEmpty())
    }

    @Test fun hardLaunchIsDetected() {
        val speeds = List(5) { 0f } + listOf(3f, 6f, 9f, 12f, 13f, 13f, 13f)
        val ev = Events.detect(track(speeds))
        assertEquals(1, ev.size)
        assertEquals(EventType.HARD_ACCEL, ev[0].type)
        assertNotNull(ev[0].offsetMs)
    }

    @Test fun normalMopedLaunchIsNotAnEvent() {
        val speeds = List(5) { 0f } + List(12) { it * 1.2f } // 1.2 m/s^2
        assertTrue(Events.detect(track(speeds)).isEmpty())
    }

    @Test fun twoSeparateBrakingManoeuvresGiveTwoEvents() {
        val brake = listOf(10f, 5f, 1f, 0f)
        val cruise = List(15) { 10f }
        val ev = Events.detect(track(cruise + brake + List(15) { 10f } + brake + List(5) { 0f }))
        assertEquals(2, ev.count { it.type == EventType.HARD_BRAKE })
    }

    // endregion
}
