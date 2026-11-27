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
import com.mopedtracker.core.EventType
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.RideEvent
import com.mopedtracker.core.RouteProjection
import com.mopedtracker.ui.theme.BadRed
import com.mopedtracker.ui.theme.GoodGreen
import com.mopedtracker.ui.theme.Teal
import com.mopedtracker.ui.theme.speedColor
import kotlin.math.max

data class RoutePoint(val lat: Double, val lon: Double, val speedMps: Float)

/** Full route drawing, each segment coloured by the speed at that point, with optional event markers. */
@Composable
fun RouteMap(
    points: List<RoutePoint>,
    maxSpeedMps: Float,
    modifier: Modifier = Modifier,
    events: List<RideEvent> = emptyList(),
) {
    if (points.size < 2) {
        Box(modifier, contentAlignment = Alignment.Center) {
            Text("No route recorded", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        return
    }
    val grid = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f)
    Canvas(modifier = modifier) {
        drawGrid(grid)
        val latLons = points.map { LatLon(it.lat, it.lon) }
        val proj = RouteProjection(latLons, size.width, size.height, 28.dp.toPx())
        val offsets = latLons.map { Offset(proj.x(it), proj.y(it)) }
        val top = max(maxSpeedMps, 1f)
        val width = 6.dp.toPx()
        // dark casing underneath for contrast
        for (i in 0 until offsets.size - 1) {
            drawLine(Color.Black.copy(alpha = 0.35f), offsets[i], offsets[i + 1], width + 3.dp.toPx(), StrokeCap.Round)
        }
        for (i in 0 until offsets.size - 1) {
            drawLine(speedColor(points[i].speedMps / top), offsets[i], offsets[i + 1], width, StrokeCap.Round)
        }
        marker(offsets.first(), GoodGreen, 9.dp.toPx(), 6.dp.toPx())
        marker(offsets.last(), BadRed, 9.dp.toPx(), 6.dp.toPx())
        for (e in events) {
            val at = Offset(proj.x(LatLon(e.lat, e.lon)), proj.y(LatLon(e.lat, e.lon)))
            marker(at, if (e.type == EventType.HARD_BRAKE) BadRed else Teal, 8.dp.toPx(), 4.5.dp.toPx(), ring = true)
        }
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
        val proj = RouteProjection(points, size.width, size.height, 8.dp.toPx())
        val offsets = points.map { Offset(proj.x(it), proj.y(it)) }
        for (i in 0 until offsets.size - 1) {
            drawLine(color, offsets[i], offsets[i + 1], strokeWidth.toPx(), StrokeCap.Round)
        }
        drawCircle(color, strokeWidth.toPx() * 1.6f, offsets.last())
    }
}

private fun DrawScope.marker(at: Offset, color: Color, outer: Float, inner: Float, ring: Boolean = false) {
    drawCircle(Color.White, outer, at)
    drawCircle(color, inner, at)
    if (ring) drawCircle(Color.Black.copy(alpha = 0.25f), outer, at, style = androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))
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
