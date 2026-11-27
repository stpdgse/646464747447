package com.mopedtracker.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.mopedtracker.core.GpsQuality
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStatus
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.theme.BadRed
import com.mopedtracker.ui.theme.GoodGreen
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.WarnAmber
import com.mopedtracker.ui.theme.speedColor
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min

private val Ink = Color(0xFF000000)
private val Digits = Color(0xFFF5F7FA)
private val Dim = Color(0xFF8A929C)

/**
 * Speedometer-only view for a handlebar mount: huge digits on pure black, readable in portrait and landscape.
 * Pure black also saves battery on OLED screens.
 */
@Composable
fun BigSpeedScreen(live: LiveState, unit: SpeedUnit, onClose: () -> Unit) {
    val paused = live.status == RideStatus.PAUSED
    val s = live.stats
    val minScale = if (unit == SpeedUnit.KMH) 60f else 40f
    val stepScale = if (unit == SpeedUnit.KMH) 20f else 10f
    val scale = max(minScale, ceil(Format.speedValue(max(live.currentSpeedMps, s.maxSpeedMps), unit) / stepScale) * stepScale)
    val fraction = (Format.speedValue(live.currentSpeedMps, unit) / scale).coerceIn(0f, 1f)

    BoxWithConstraints(Modifier.fillMaxSize().background(Ink)) {
        val density = LocalDensity.current
        // three digits must fit the width, and the number shouldn't swallow the height in landscape
        val fontDp = min(maxWidth.value * 0.5f, maxHeight.value * 0.42f)
        val fontSize = with(density) { fontDp.dp.toSp() }

        Column(
            Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding().padding(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                val (dot, label) = when {
                    paused -> WarnAmber to "PAUSED"
                    live.gps == GpsQuality.GOOD -> GoodGreen to "GPS ±${live.accuracyM?.toInt()} m"
                    live.gps == GpsQuality.OK -> WarnAmber to "GPS ±${live.accuracyM?.toInt()} m"
                    live.gps == GpsQuality.WEAK -> BadRed to "GPS weak"
                    else -> WarnAmber to "Searching GPS…"
                }
                Canvas(Modifier.size(12.dp)) { drawCircle(dot) }
                Text("  $label", color = Dim, fontSize = 15.sp, modifier = Modifier.weight(1f))
                IconButton(onClick = onClose) { Icon(Icons.Filled.Close, "Close", tint = Dim) }
            }

            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    Format.speed(live.currentSpeedMps, unit),
                    color = if (paused) Dim else Digits,
                    fontSize = fontSize,
                    fontWeight = FontWeight.Bold,
                    lineHeight = fontSize,
                    maxLines = 1,
                    softWrap = false,
                    textAlign = TextAlign.Center,
                )
                Text(unit.label, color = Dim, fontSize = (fontDp * 0.16f).coerceAtLeast(18f).sp, fontWeight = FontWeight.Medium)
            }

            Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
                SpeedBar(fraction)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Mini("MAX", "${Format.speed(s.maxSpeedMps, unit)} ${unit.label}", Orange)
                    Mini("DISTANCE", "${Format.distance(s.distanceM, unit)} ${unit.distanceLabel}", Digits)
                    Mini("TIME", Format.duration(s.elapsedMs), Digits)
                }
            }
        }
    }
}

@Composable
private fun SpeedBar(fraction: Float) {
    Canvas(Modifier.fillMaxWidth().height(14.dp)) {
        val segments = 40
        val gap = 4.dp.toPx()
        val w = (size.width - gap * (segments - 1)) / segments
        val lit = (fraction * segments).toInt()
        for (i in 0 until segments) {
            drawRoundRect(
                color = if (i < lit) speedColor(i / (segments - 1f)) else Color(0xFF1A1D22),
                topLeft = Offset(i * (w + gap), 0f),
                size = Size(w, size.height),
                cornerRadius = CornerRadius(4.dp.toPx()),
            )
        }
    }
}

@Composable
private fun Mini(label: String, value: String, color: Color) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(label, color = Dim, fontSize = 12.sp, fontWeight = FontWeight.Medium, letterSpacing = 1.sp)
        Text(value, color = color, fontSize = 22.sp, fontWeight = FontWeight.Bold)
    }
}
