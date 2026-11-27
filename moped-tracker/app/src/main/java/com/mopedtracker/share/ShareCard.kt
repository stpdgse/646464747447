package com.mopedtracker.share

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Shader
import android.graphics.Typeface
import androidx.compose.ui.graphics.toArgb
import androidx.core.content.FileProvider
import com.mopedtracker.core.Analysis
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.RouteProjection
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.theme.speedColor
import java.io.File
import kotlin.math.max

/** Draws a shareable 1080x1350 summary card of a ride with plain Android graphics. */
object ShareCard {
    const val WIDTH = 1080
    const val HEIGHT = 1350

    private const val ORANGE = 0xFFFF8A3D.toInt()
    private const val WHITE = 0xFFF2F4F7.toInt()
    private const val MUTED = 0xFF9AA3AE.toInt()
    private const val PANEL = 0xFF171A1F.toInt()
    private val bold = Typeface.create("sans-serif", Typeface.BOLD)
    private val medium = Typeface.create("sans-serif-medium", Typeface.NORMAL)
    private val regular = Typeface.create("sans-serif", Typeface.NORMAL)

    fun render(session: SessionEntity, points: List<TrackPointEntity>, unit: SpeedUnit, vehicleName: String? = null): Bitmap {
        val bmp = Bitmap.createBitmap(WIDTH, HEIGHT, Bitmap.Config.ARGB_8888)
        val c = Canvas(bmp)

        // background
        val bg = Paint().apply {
            shader = LinearGradient(0f, 0f, 0f, HEIGHT.toFloat(), 0xFF0E1013.toInt(), 0xFF1D222B.toInt(), Shader.TileMode.CLAMP)
        }
        c.drawRect(0f, 0f, WIDTH.toFloat(), HEIGHT.toFloat(), bg)

        // header
        text(c, "MOPED TRACKER", 70f, 110f, 34f, ORANGE, bold, letterSpacing = 0.18f)
        text(c, fit(session.name, 940f, 74f, bold), 70f, 205f, 74f, WHITE, bold)
        text(c, fit(listOfNotNull(Format.dateTime(session.startTime), vehicleName).joinToString(" · "), 940f, 38f, regular), 70f, 262f, 38f, MUTED, regular)

        // route panel
        val panel = RectF(60f, 310f, 1020f, 880f)
        c.drawRoundRect(panel, 48f, 48f, Paint(Paint.ANTI_ALIAS_FLAG).apply { color = PANEL })
        drawRoute(c, panel, points, session.maxSpeedMps)

        // speed legend under the route
        val legendY = 850f
        val legendLeft = 130f
        val legendRight = 950f
        val steps = 40
        val seg = (legendRight - legendLeft) / steps
        val lp = Paint()
        for (i in 0 until steps) {
            lp.color = speedColor(i / (steps - 1f)).toArgb()
            c.drawRect(legendLeft + i * seg, legendY - 6f, legendLeft + (i + 1) * seg + 1f, legendY + 6f, lp)
        }

        // stats
        val cells = listOf(
            Triple("DISTANCE", Format.distance(session.distanceM, unit), unit.distanceLabel),
            Triple("MAX SPEED", Format.speed(session.maxSpeedMps, unit), unit.label),
            Triple("DURATION", Format.duration(session.elapsedMs), ""),
            Triple("AVG SPEED", Format.speed(session.avgMovingSpeedMps, unit, 1), unit.label),
        )
        cells.forEachIndexed { i, (label, value, u) ->
            val col = i % 2
            val row = i / 2
            val x = 70f + col * 490f
            val y = 960f + row * 190f
            text(c, label, x, y, 30f, MUTED, medium, letterSpacing = 0.12f)
            val vp = Paint(Paint.ANTI_ALIAS_FLAG).apply { textSize = 92f; typeface = bold; color = if (i == 1) ORANGE else WHITE }
            c.drawText(value, x, y + 98f, vp)
            if (u.isNotEmpty()) text(c, u, x + vp.measureText(value) + 14f, y + 98f, 38f, MUTED, regular)
        }

        // footer
        text(c, "Tracked with Moped Tracker", WIDTH / 2f, 1310f, 30f, MUTED, regular, align = Paint.Align.CENTER)
        return bmp
    }

    private fun drawRoute(c: Canvas, panel: RectF, all: List<TrackPointEntity>, maxSpeed: Float) {
        val pts = Analysis.thin(all, 700)
        if (pts.size < 2) {
            text(c, "No route recorded", panel.centerX(), panel.centerY(), 40f, MUTED, regular, align = Paint.Align.CENTER)
            return
        }
        val inner = 56f
        val w = panel.width() - 2 * inner
        val h = panel.height() - 2 * inner - 40f // leave room for the legend
        val proj = RouteProjection(pts.map { LatLon(it.lat, it.lon) }, w, h, 20f)
        val ox = panel.left + inner
        val oy = panel.top + inner
        fun px(p: TrackPointEntity) = ox + proj.x(LatLon(p.lat, p.lon))
        fun py(p: TrackPointEntity) = oy + proj.y(LatLon(p.lat, p.lon))

        val casing = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0x66000000; strokeWidth = 20f; strokeCap = Paint.Cap.ROUND }
        val line = Paint(Paint.ANTI_ALIAS_FLAG).apply { strokeWidth = 13f; strokeCap = Paint.Cap.ROUND }
        for (i in 0 until pts.size - 1) c.drawLine(px(pts[i]), py(pts[i]), px(pts[i + 1]), py(pts[i + 1]), casing)
        val top = max(maxSpeed, 1f)
        for (i in 0 until pts.size - 1) {
            line.color = speedColor(pts[i].speedMps / top).toArgb()
            c.drawLine(px(pts[i]), py(pts[i]), px(pts[i + 1]), py(pts[i + 1]), line)
        }
        marker(c, px(pts.first()), py(pts.first()), 0xFF4ADE80.toInt())
        marker(c, px(pts.last()), py(pts.last()), 0xFFFF5C5C.toInt())
    }

    private fun marker(c: Canvas, x: Float, y: Float, color: Int) {
        c.drawCircle(x, y, 22f, Paint(Paint.ANTI_ALIAS_FLAG).apply { this.color = 0xFFFFFFFF.toInt() })
        c.drawCircle(x, y, 14f, Paint(Paint.ANTI_ALIAS_FLAG).apply { this.color = color })
    }

    private fun text(
        c: Canvas, s: String, x: Float, y: Float, size: Float, color: Int, face: Typeface,
        letterSpacing: Float = 0f, align: Paint.Align = Paint.Align.LEFT,
    ) {
        val p = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            textSize = size; this.color = color; typeface = face; this.letterSpacing = letterSpacing; textAlign = align
        }
        c.drawText(s, x, y, p)
    }

    /** Shortens [s] with an ellipsis until it fits [maxWidth]. */
    private fun fit(s: String, maxWidth: Float, size: Float, face: Typeface): String {
        val p = Paint().apply { textSize = size; typeface = face }
        if (p.measureText(s) <= maxWidth) return s
        var t = s
        while (t.isNotEmpty() && p.measureText("$t…") > maxWidth) t = t.dropLast(1)
        return "$t…"
    }

    /** Saves the card as a PNG in the cache and opens the Android share sheet. */
    fun share(context: Context, bitmap: Bitmap, rideId: Long, title: String) {
        val dir = File(context.cacheDir, "share").apply { mkdirs() }
        val file = File(dir, "ride_$rideId.png")
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "image/png"
            putExtra(Intent.EXTRA_STREAM, uri)
            putExtra(Intent.EXTRA_TEXT, title)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(send, "Share ride").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
}
