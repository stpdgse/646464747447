package com.mopedtracker

import com.mopedtracker.core.Fix
import com.mopedtracker.core.StatsAccumulator
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class StatsAccumulatorTest {

    private val metresPerDegLat = 111_195.0

    /** A fix [secs] seconds into a ride heading due north at a constant [speed] m/s. */
    private fun northFix(secs: Int, speed: Float, accuracy: Float = 4f, speedAcc: Float? = 0.5f, alt: Double? = null) =
        Fix(
            timeMs = secs * 1000L,
            lat = 48.0 + speed * secs / metresPerDegLat,
            lon = 11.0,
            altitudeM = alt,
            speedMps = speed,
            accuracyM = accuracy,
            speedAccuracyMps = speedAcc,
        )

    @Test
    fun constantSpeedDriveGivesExpectedDistanceAndAverages() {
        val acc = StatsAccumulator()
        for (s in 0..60) acc.add(northFix(s, 10f)) // 36 km/h for a minute
        val st = acc.snapshot(elapsedMs = 60_000)

        assertEquals(600.0, st.distanceM, 6.0)
        assertEquals(10f, st.maxSpeedMps, 0.01f)
        assertEquals(10f, st.avgSpeedMps, 0.2f)
        assertEquals(10f, st.avgMovingSpeedMps, 0.2f)
        assertEquals(60_000, st.movingMs)
        assertEquals(0, st.stoppedMs)
    }

    @Test
    fun standingStillWithGpsDriftAddsNoDistanceAndLowersOnlyTheOverallAverage() {
        val acc = StatsAccumulator()
        for (s in 0..30) acc.add(northFix(s, 10f))           // drive 300 m
        val base = acc.snapshot(30_000)
        // 30 s at a red light: speed ~0 but position jitters by a few metres.
        for (s in 31..60) {
            acc.add(Fix(s * 1000L, 48.0 + (300 + (s % 3)) / metresPerDegLat, 11.0 + (s % 2) * 3e-5,
                speedMps = 0.1f, accuracyM = 5f, speedAccuracyMps = 0.3f))
        }
        val st = acc.snapshot(60_000)

        assertEquals(base.distanceM, st.distanceM, 0.001)
        assertEquals(30_000, st.stoppedMs, 1500)
        assertTrue(st.avgMovingSpeedMps > st.avgSpeedMps)
        assertEquals(10f, st.avgMovingSpeedMps, 0.3f)
    }

    @Test
    fun implausibleSpeedSpikeIsRejectedAndDoesNotBecomeTheMaxSpeed() {
        val acc = StatsAccumulator()
        for (s in 0..10) acc.add(northFix(s, 12f))
        val spike = acc.add(northFix(11, 55f)) // +43 m/s in one second is not a moped
        for (s in 12..20) acc.add(northFix(s, 12f))
        val st = acc.snapshot(20_000)

        assertFalse(spike.accepted)
        assertEquals(12f, st.maxSpeedMps, 0.01f)
    }

    @Test
    fun poorAccuracyFixesAreIgnored() {
        val acc = StatsAccumulator()
        acc.add(northFix(0, 10f))
        val bad = acc.add(northFix(1, 10f, accuracy = 80f))
        assertFalse(bad.accepted)
        assertEquals(1, acc.snapshot(1000).pointCount)
    }

    @Test
    fun maxSpeedRequiresGoodSpeedAccuracy() {
        val acc = StatsAccumulator()
        for (s in 0..5) acc.add(northFix(s, 10f))
        acc.add(northFix(6, 14f, speedAcc = 4f)) // accepted for distance but too uncertain for "max"
        assertEquals(10f, acc.snapshot(6000).maxSpeedMps, 0.01f)
    }

    @Test
    fun speedIsDerivedFromPositionWhenProviderHasNone() {
        val acc = StatsAccumulator()
        for (s in 0..20) {
            acc.add(Fix(s * 1000L, 48.0 + 8.0 * s / metresPerDegLat, 11.0, speedMps = null, accuracyM = 4f))
        }
        assertEquals(8f, acc.snapshot(20_000).maxSpeedMps, 0.3f)
    }

    @Test
    fun elevationGainUsesHysteresisSoNoiseDoesNotCount() {
        val acc = StatsAccumulator()
        val alts = listOf(100.0, 100.8, 99.6, 100.5, 99.9, 100.4) // +-1 m noise
        alts.forEachIndexed { i, a -> acc.add(northFix(i, 10f, alt = a)) }
        assertEquals(0.0, acc.snapshot(5000).elevationGainM, 0.001)

        val climb = StatsAccumulator()
        listOf(100.0, 102.0, 104.0, 106.0, 108.0, 110.0).forEachIndexed { i, a -> climb.add(northFix(i, 10f, alt = a)) }
        val st = climb.snapshot(5000)
        assertEquals(10.0, st.elevationGainM, 4.1) // hysteresis may lag by up to one threshold
        assertEquals(110.0, st.maxAltitudeM!!, 0.001)
        assertEquals(100.0, st.minAltitudeM!!, 0.001)
    }

    @Test
    fun afterBreakLinkTheGapIsNotCountedAsDistance() {
        val acc = StatsAccumulator()
        for (s in 0..10) acc.add(northFix(s, 10f))
        val before = acc.snapshot(10_000).distanceM
        acc.breakLink() // pause + resume 5 km further along
        for (s in 600..610) {
            acc.add(Fix(s * 1000L, 48.05 + 10.0 * (s - 600) / metresPerDegLat, 11.0, speedMps = 10f, accuracyM = 4f))
        }
        val after = acc.snapshot(20_000).distanceM
        assertEquals(before + 100.0, after, 3.0)
    }
}
