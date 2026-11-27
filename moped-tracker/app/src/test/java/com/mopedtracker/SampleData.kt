package com.mopedtracker

import com.mopedtracker.core.Fix
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.StatsAccumulator
import com.mopedtracker.data.RouteCodec
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.data.toFix
import kotlin.math.cos
import kotlin.math.sin

/** Builds a believable ~14 minute moped ride so screenshots show realistic numbers. */
object SampleData {
    private const val START = 1_764_230_400_000L // 2025-11-27 08:00 UTC

    fun points(sessionId: Long = 1, seed: Double = 0.0): List<TrackPointEntity> {
        val out = ArrayList<TrackPointEntity>()
        var lat = 48.1371
        var lon = 11.5754
        var heading = 0.6 + seed
        var t = START
        for (i in 0 until 840) {
            val phase = i / 840.0
            // cruise 35-48 km/h with a few red-light stops and one fast stretch
            var v = 10.0 + 3.0 * sin(i / 40.0) + 1.5 * sin(i / 9.0)
            if (i in 180..205 || i in 470..500 || i in 700..715) v = 0.2
            if (i in 520..600) v += 3.0
            v = v.coerceAtLeast(0.0)
            heading += 0.012 * sin(i / 55.0) + 0.006 * cos(i / 23.0 + seed)
            lat += v * cos(heading) / 111_195.0
            lon += v * sin(heading) / (111_195.0 * cos(Math.toRadians(lat)))
            out += TrackPointEntity(
                sessionId = sessionId, time = t, lat = lat, lon = lon,
                altitude = 520.0 + 18.0 * sin(phase * 6.0) + 6.0 * phase,
                speedMps = v.toFloat(), accuracyM = 4f, speedAccuracyMps = 0.4f, bearing = null,
            )
            t += 1000
        }
        return out
    }

    fun session(id: Long = 1, name: String = "Morning commute", seed: Double = 0.0): Pair<SessionEntity, List<TrackPointEntity>> {
        val pts = points(id, seed)
        val acc = StatsAccumulator()
        pts.forEach { acc.add(it.toFix()) }
        val elapsed = pts.last().time - pts.first().time
        val st = acc.snapshot(elapsed)
        val s = SessionEntity(
            id = id, name = name, startTime = pts.first().time, endTime = pts.last().time,
            distanceM = st.distanceM, elapsedMs = st.elapsedMs, movingMs = st.movingMs,
            maxSpeedMps = st.maxSpeedMps, avgSpeedMps = st.avgSpeedMps, avgMovingSpeedMps = st.avgMovingSpeedMps,
            elevationGainM = st.elevationGainM, elevationLossM = st.elevationLossM,
            maxAltitudeM = st.maxAltitudeM, minAltitudeM = st.minAltitudeM,
            avgAccuracyM = st.avgAccuracyM, pointCount = st.pointCount,
            previewRoute = RouteCodec.encode(RouteCodec.downsample(pts.map { LatLon(it.lat, it.lon) }, 60)),
        )
        return s to pts
    }

    @Suppress("unused")
    fun fixes(points: List<TrackPointEntity>): List<Fix> = points.map { it.toFix() }
}
