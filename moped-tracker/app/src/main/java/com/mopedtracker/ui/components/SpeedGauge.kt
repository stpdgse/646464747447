package com.mopedtracker.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import com.mopedtracker.ui.theme.speedColor

private const val START_ANGLE = 150f
private const val SWEEP = 240f

/**
 * Big arc speedometer. [value] and [max] are in the display unit (km/h or mph).
 * The arc is painted in small segments so the colour ramps from blue to red as the speed rises.
 */
@Composable
fun SpeedGauge(
    value: Float,
    max: Float,
    unitLabel: String,
    valueText: String,
    modifier: Modifier = Modifier,
) {
    val track = MaterialTheme.colorScheme.surfaceVariant
    Box(modifier = modifier.fillMaxWidth().aspectRatio(1f), contentAlignment = Alignment.Center) {
        Canvas(modifier = Modifier.matchParentSize()) {
            val stroke = 22.dp.toPx()
            val inset = stroke / 2 + 4.dp.toPx()
            val arcSize = Size(size.width - 2 * inset, size.height - 2 * inset)
            val topLeft = Offset(inset, inset)
            drawArc(
                color = track, startAngle = START_ANGLE, sweepAngle = SWEEP, useCenter = false,
                topLeft = topLeft, size = arcSize, style = Stroke(stroke, cap = StrokeCap.Round),
            )
            val fraction = (value / max).coerceIn(0f, 1f)
            val segments = 96
            val lit = (fraction * segments).toInt()
            for (i in 0 until lit) {
                val a = START_ANGLE + SWEEP * i / segments
                drawArc(
                    color = speedColor(i.toFloat() / segments),
                    startAngle = a, sweepAngle = SWEEP / segments + 0.6f, useCenter = false,
                    topLeft = topLeft, size = arcSize, style = Stroke(stroke, cap = StrokeCap.Butt),
                )
            }
        }
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(valueText, style = MaterialTheme.typography.displayLarge, color = MaterialTheme.colorScheme.onBackground)
            Text(unitLabel, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
