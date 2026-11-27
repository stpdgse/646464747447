package com.mopedtracker.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.HistogramBar
import com.mopedtracker.ui.theme.speedColor

/** Speed-over-time line chart. [values] are in the display unit, evenly spaced in time. */
@Composable
fun SpeedChart(values: List<Float>, unitLabel: String, durationLabel: String, modifier: Modifier = Modifier) {
    if (values.size < 2) return
    val top = niceCeil(values.max())
    val line = MaterialTheme.colorScheme.primary
    val grid = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    Column(modifier) {
        Row(Modifier.fillMaxWidth().height(150.dp)) {
            Column(
                Modifier.fillMaxHeight().padding(end = 8.dp),
                verticalArrangement = Arrangement.SpaceBetween,
                horizontalAlignment = Alignment.End,
            ) {
                Text("${top.toInt()}", style = MaterialTheme.typography.labelMedium, color = muted)
                Text("${(top / 2).toInt()}", style = MaterialTheme.typography.labelMedium, color = muted)
                Text("0", style = MaterialTheme.typography.labelMedium, color = muted)
            }
            Canvas(Modifier.fillMaxSize()) {
                for (f in listOf(0f, 0.5f, 1f)) {
                    val y = size.height * (1f - f)
                    drawLine(grid, Offset(0f, y), Offset(size.width, y), 1f)
                }
                val stepX = size.width / (values.size - 1)
                val path = Path()
                val fill = Path()
                values.forEachIndexed { i, v ->
                    val x = i * stepX
                    val y = size.height * (1f - (v / top).coerceIn(0f, 1f))
                    if (i == 0) { path.moveTo(x, y); fill.moveTo(x, size.height); fill.lineTo(x, y) }
                    else { path.lineTo(x, y); fill.lineTo(x, y) }
                }
                fill.lineTo(size.width, size.height)
                fill.close()
                drawPath(fill, Brush.verticalGradient(listOf(line.copy(alpha = 0.35f), line.copy(alpha = 0f))))
                drawPath(path, line, style = Stroke(2.5.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round))
                val maxIdx = values.indices.maxBy { values[it] }
                val mx = maxIdx * stepX
                val my = size.height * (1f - (values[maxIdx] / top).coerceIn(0f, 1f))
                drawCircle(line, 5.dp.toPx(), Offset(mx, my))
                drawCircle(androidx.compose.ui.graphics.Color.White, 2.dp.toPx(), Offset(mx, my))
            }
        }
        Row(Modifier.fillMaxWidth().padding(start = 28.dp, top = 4.dp), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("0:00", style = MaterialTheme.typography.labelMedium, color = muted)
            Text(unitLabel, style = MaterialTheme.typography.labelMedium, color = muted)
            Text(durationLabel, style = MaterialTheme.typography.labelMedium, color = muted)
        }
    }
}

/** Horizontal bars: share of ride time spent in each speed band. */
@Composable
fun SpeedHistogram(bars: List<HistogramBar>, unitLabel: String, modifier: Modifier = Modifier) {
    if (bars.isEmpty()) return
    val peak = bars.maxOf { it.fraction }.coerceAtLeast(0.01f)
    val maxTo = bars.last().toUnit.toFloat()
    Column(modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        bars.forEach { bar ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "${bar.fromUnit}–${bar.toUnit}",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.width(60.dp),
                )
                Box(Modifier.weight(1f).height(14.dp)) {
                    Box(
                        Modifier
                            .fillMaxHeight()
                            .fillMaxWidth((bar.fraction / peak).coerceAtLeast(0.01f))
                            .background(speedColor(bar.fromUnit / maxTo), RoundedCornerShape(7.dp)),
                    )
                }
                Text(
                    "${(bar.fraction * 100).toInt()}%",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.width(44.dp).padding(start = 8.dp),
                )
            }
        }
        Text("Time spent per speed band ($unitLabel)", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

private fun niceCeil(v: Float): Float {
    val step = if (v <= 40f) 10f else 20f
    return (kotlin.math.ceil(v / step) * step).coerceAtLeast(step)
}
