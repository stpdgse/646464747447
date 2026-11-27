package com.mopedtracker.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.LatLon
import com.mopedtracker.ui.theme.GoodGreen
import com.mopedtracker.ui.theme.BadRed
import com.mopedtracker.ui.theme.speedColor
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min

data class RoutePoint(val lat: Double, val lon: Double, val speedMps: Float)

/** Fits lat/lon points into a canvas, keeping the real-world aspect ratio. */
private class Projector(points: List<LatLon>, width: Float, height: Float, pad: Float) {
    private val minLat = points.minOf { it.lat }
    private val maxLat = points.maxOf { it.lat }
    private val minLon = points.minOf { it.lon }
    private val maxLon = points.maxOf { it.lon }
    private val lonScale = cos(Math.toRadians((minLat + maxLat) / 2))
    private val spanX = max((maxLon - minLon) * lonScale, 1e-6)
    private val spanY = max(maxLat - minLat, 1e-6)
    private val scale = min((width - 2 * pad) / spanX, (height - 2 * pad) / spanY)
    private val offX = (width - spanX * scale) / 2
    private val offY = (height - spanY * scale) / 2

    fun project(p: LatLon): Offset = Offset(
        (offX + (p.lon - minLon) * lonScale * scale).toFloat(),
        (height - (offY + (p.lat - minLat) * scale)).toFloat(),
    )
}

/** Full route drawing, each segment coloured by the speed at that point. */
@Composable
fun RouteMap(points: List<RoutePoint>, maxSpeedMps: Float, modifier: Modifier = Modifier) {
    if (points.size < 2) {
        Box(modifier, contentAlignment = Alignment.Center) {
            Text("No route recorded", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        return
    }
    val grid = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f)
    Canvas(modifier = modifier) {
        drawGrid(grid)
        val proj = Projector(points.map { LatLon(it.lat, it.lon) }, size.width, size.height, 28.dp.toPx())
        val offsets = points.map { proj.project(LatLon(it.lat, it.lon)) }
        val top = max(maxSpeedMps, 1f)
        val width = 6.dp.toPx()
        // dark casing underneath for contrast
        for (i in 0 until offsets.size - 1) {
            drawLine(Color.Black.copy(alpha = 0.35f), offsets[i], offsets[i + 1], width + 3.dp.toPx(), StrokeCap.Round)
        }
        for (i in 0 until offsets.size - 1) {
            drawLine(speedColor(points[i].speedMps / top), offsets[i], offsets[i + 1], width, StrokeCap.Round)
        }
        marker(offsets.first(), GoodGreen)
        marker(offsets.last(), BadRed)
    }
}

/** Compact single-colour route, used for list thumbnails and the live view. */
@Composable
fun RoutePreview(points: List<LatLon>, color: Color, modifier: Modifier = Modifier, strokeWidth: Dp = 3.dp) {
    if (points.size < 2) {
        Box(modifier.fillMaxSize())
        return
    }
    Canvas(modifier = modifier) {
        val proj = Projector(points, size.width, size.height, 8.dp.toPx())
        val offsets = points.map { proj.project(it) }
        for (i in 0 until offsets.size - 1) {
            drawLine(color, offsets[i], offsets[i + 1], strokeWidth.toPx(), StrokeCap.Round)
        }
        drawCircle(color, strokeWidth.toPx() * 1.6f, offsets.last())
    }
}

private fun DrawScope.marker(at: Offset, color: Color) {
    drawCircle(Color.White, 9.dp.toPx(), at)
    drawCircle(color, 6.dp.toPx(), at)
}

private fun DrawScope.drawGrid(color: Color) {
    val step = 40.dp.toPx()
    var x = 0f
    while (x < size.width) {
        drawLine(color, Offset(x, 0f), Offset(x, size.height), 1f)
        x += step
    }
    var y = 0f
    while (y < size.height) {
        drawLine(color, Offset(0f, y), Offset(size.width, y), 1f)
        y += step
    }
}
