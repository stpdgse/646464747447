package com.mopedtracker.data

import com.mopedtracker.core.Fix
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.RideStats
import com.mopedtracker.core.StatsAccumulator
import java.text.DateFormat
import java.util.Date

class RideRepository(private val dao: RideDao) {

    fun observeFinished() = dao.observeFinished()
    fun observeSession(id: Long) = dao.observeSession(id)
    suspend fun getSession(id: Long) = dao.getSession(id)
    suspend fun getPoints(id: Long) = dao.getPoints(id)
    suspend fun getActiveSessionOrNull() = dao.getActiveSessions().firstOrNull()
    suspend fun delete(id: Long) = dao.deleteSession(id)
    suspend fun rename(id: Long, name: String) = dao.rename(id, name.trim().ifEmpty { "Ride" })

    suspend fun createSession(startTime: Long): Long {
        val label = DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date(startTime))
        return dao.insertSession(SessionEntity(name = "Ride $label", startTime = startTime))
    }

    suspend fun addPoint(sessionId: Long, fix: Fix, speedMps: Float) = dao.insertPoint(
        TrackPointEntity(
            sessionId = sessionId, time = fix.timeMs, lat = fix.lat, lon = fix.lon,
            altitude = fix.altitudeM, speedMps = speedMps, accuracyM = fix.accuracyM,
            speedAccuracyMps = fix.speedAccuracyMps, bearing = fix.bearingDeg,
        ),
    )

    suspend fun savePausedMs(sessionId: Long, pausedMs: Long) {
        val s = dao.getSession(sessionId) ?: return
        dao.updateSession(s.copy(pausedMs = pausedMs))
    }

    /** Writes the final statistics and marks the ride as finished. Rides without data are discarded. */
    suspend fun finish(sessionId: Long, endTime: Long, pausedMs: Long, stats: RideStats) {
        val s = dao.getSession(sessionId) ?: return
        if (stats.pointCount < 2) {
            dao.deleteSession(sessionId)
            return
        }
        val points = dao.getPoints(sessionId)
        dao.updateSession(
            s.copy(
                endTime = endTime,
                pausedMs = pausedMs,
                distanceM = stats.distanceM,
                elapsedMs = stats.elapsedMs,
                movingMs = stats.movingMs,
                maxSpeedMps = stats.maxSpeedMps,
                avgSpeedMps = stats.avgSpeedMps,
                avgMovingSpeedMps = stats.avgMovingSpeedMps,
                elevationGainM = stats.elevationGainM,
                elevationLossM = stats.elevationLossM,
                maxAltitudeM = stats.maxAltitudeM,
                minAltitudeM = stats.minAltitudeM,
                avgAccuracyM = stats.avgAccuracyM,
                pointCount = stats.pointCount,
                previewRoute = RouteCodec.encode(RouteCodec.downsample(points.map { LatLon(it.lat, it.lon) }, 60)),
            ),
        )
    }

    /** Closes rides left open by a crash or killed process, rebuilding their stats from stored points. */
    suspend fun finalizeOrphans(exceptId: Long? = null) {
        for (s in dao.getActiveSessions()) {
            if (s.id == exceptId) continue
            val points = dao.getPoints(s.id)
            val acc = StatsAccumulator()
            points.forEach { acc.add(it.toFix()) }
            val end = points.lastOrNull()?.time ?: s.startTime
            val elapsed = (end - s.startTime - s.pausedMs).coerceAtLeast(0)
            finish(s.id, end, s.pausedMs, acc.snapshot(elapsed))
        }
    }
}

fun TrackPointEntity.toFix() = Fix(
    timeMs = time, lat = lat, lon = lon, altitudeM = altitude, speedMps = speedMps,
    accuracyM = accuracyM, speedAccuracyMps = speedAccuracyMps, bearingDeg = bearing,
)

object RouteCodec {
    fun encode(points: List<LatLon>): String =
        points.joinToString(";") { "%.5f,%.5f".format(java.util.Locale.US, it.lat, it.lon) }

    fun decode(s: String): List<LatLon> =
        if (s.isBlank()) emptyList()
        else s.split(';').mapNotNull {
            val p = it.split(',')
            if (p.size == 2) p[0].toDoubleOrNull()?.let { la -> p[1].toDoubleOrNull()?.let { lo -> LatLon(la, lo) } } else null
        }

    fun <T> downsample(list: List<T>, max: Int): List<T> {
        if (list.size <= max || max < 2) return list
        val step = (list.size - 1).toDouble() / (max - 1)
        return List(max) { list[(it * step).toInt()] }
    }
}
