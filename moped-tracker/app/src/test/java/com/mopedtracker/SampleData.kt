package com.mopedtracker

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Shader
import com.mopedtracker.core.Fix
import com.mopedtracker.data.VehicleEntity
import java.io.File
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

    /** Draws a simple stylised scooter photo and saves it as a real JPEG, so the app's photo decoding is exercised. */
    fun photoFile(context: Context, name: String, skyTop: Int, skyBottom: Int, body: Int): String {
        val w = 1024
        val h = 768
        val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val c = Canvas(bmp)
        val p = Paint(Paint.ANTI_ALIAS_FLAG)
        p.shader = LinearGradient(0f, 0f, 0f, h * 0.72f, skyTop, skyBottom, Shader.TileMode.CLAMP)
        c.drawRect(0f, 0f, w.toFloat(), h * 0.72f, p)
        p.shader = null
        p.color = 0xFF2A2D33.toInt()
        c.drawRect(0f, h * 0.72f, w.toFloat(), h.toFloat(), p)
        // wheels
        p.color = 0xFF111111.toInt()
        c.drawCircle(300f, 540f, 105f, p)
        c.drawCircle(730f, 540f, 105f, p)
        p.color = 0xFFB9BEC6.toInt()
        c.drawCircle(300f, 540f, 48f, p)
        c.drawCircle(730f, 540f, 48f, p)
        // body
        p.color = body
        c.drawRoundRect(RectF(270f, 330f, 790f, 520f), 90f, 90f, p)
        c.drawRoundRect(RectF(600f, 220f, 770f, 440f), 60f, 60f, p)
        p.color = 0xFF1B1B1B.toInt()
        c.drawRoundRect(RectF(330f, 300f, 600f, 360f), 30f, 30f, p)
        p.strokeWidth = 16f
        p.color = 0xFF222222.toInt()
        c.drawLine(700f, 230f, 640f, 150f, p)
        c.drawLine(600f, 150f, 690f, 150f, p)
        val dir = File(context.cacheDir, "sample-photos").apply { mkdirs() }
        val f = File(dir, "$name.jpg")
        f.outputStream().use { bmp.compress(Bitmap.CompressFormat.JPEG, 90, it) }
        return f.absolutePath
    }

    fun vehicles(context: Context): List<VehicleEntity> = listOf(
        VehicleEntity(
            id = 1, name = "Vespa GTS 125", kind = "Scooter", hp = 11.5, torqueNm = 10.4, displacementCc = 125.0, weightKg = 148.0,
            year = 2021, tankLiters = 8.5, claimedTopKmh = 95.0, notes = "Daily commuter",
            photoPath = photoFile(context, "vespa", 0xFF2B4C7E.toInt(), 0xFFB7D4F0.toInt(), 0xFFD9423B.toInt()),
        ),
        VehicleEntity(
            id = 2, name = "Peugeot Speedfight", kind = "Moped", hp = 4.0, torqueNm = 5.1, displacementCc = 49.0, weightKg = 98.0,
            year = 2016, tankLiters = 7.0, claimedTopKmh = 45.0,
            photoPath = photoFile(context, "speedfight", 0xFF3A2C5E.toInt(), 0xFFF2C5A0.toInt(), 0xFF2E8B57.toInt()),
        ),
        VehicleEntity(id = 3, name = "Old Simson", kind = "Moped", hp = 3.2, year = 1985),
    )

    private const val START = 1_764_230_400_000L // 2025-11-27 08:00 UTC

    fun points(sessionId: Long = 1, seed: Double = 0.0): List<TrackPointEntity> {
        val out = ArrayList<TrackPointEntity>()
        var lat = 48.1371
        var lon = 11.5754
        var heading = 0.6 + seed
        var t = START
        var actual = 0.0
        for (i in 0 until 840) {
            val phase = i / 840.0
            // target: cruise 35-48 km/h with a few red-light stops and one fast stretch
            var target = 10.0 + 3.0 * sin(i / 40.0) + 1.5 * sin(i / 9.0)
            if (i in 180..205 || i in 470..500 || i in 700..715) target = 0.0
            if (i in 520..600) target += 3.0
            target = target.coerceAtLeast(0.0)
            // a real moped can't change speed instantly: limit launch and braking rates (m/s^2)
            actual = if (target > actual) minOf(target, actual + 1.6) else maxOf(target, actual - 3.4)
            val v = actual
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

    fun session(id: Long = 1, name: String = "Morning commute", seed: Double = 0.0, starred: Boolean = false): Pair<SessionEntity, List<TrackPointEntity>> {
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
            starred = starred,
        )
        return s to pts
    }

    @Suppress("unused")
    fun fixes(points: List<TrackPointEntity>): List<Fix> = points.map { it.toFix() }
}
