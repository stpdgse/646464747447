package com.mopedtracker.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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

// The 240 degree arc never reaches the bottom quarter of its circle, so the box is cropped to this ratio.
private const val HEIGHT_RATIO = 0.80f

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
    BoxWithConstraints(modifier = modifier.fillMaxWidth().aspectRatio(1f / HEIGHT_RATIO)) {
        val diameter = maxWidth
        Canvas(modifier = Modifier.matchParentSize()) {
            val stroke = 22.dp.toPx()
            val inset = stroke / 2 + 4.dp.toPx()
            val d = size.width - 2 * inset
            val arcSize = Size(d, d)
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
        // Centre the readout on the circle's centre, not the cropped box's centre.
        Column(
            Modifier.align(Alignment.TopCenter).padding(top = diameter / 2 - 66.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(valueText, style = MaterialTheme.typography.displayLarge, color = MaterialTheme.colorScheme.onBackground)
            Text(unitLabel, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
