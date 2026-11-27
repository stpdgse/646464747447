package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStatus
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.GpsChip
import com.mopedtracker.ui.components.RoutePreview
import com.mopedtracker.ui.components.SpeedGauge
import com.mopedtracker.ui.components.StatGrid
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange
import kotlin.math.ceil
import kotlin.math.max

@Composable
fun LiveScreen(
    live: LiveState,
    unit: SpeedUnit,
    onPauseResume: () -> Unit,
    onStop: () -> Unit,
    onBack: () -> Unit,
    onBigMode: () -> Unit = {},
) {
    var confirmStop by remember { mutableStateOf(false) }
    val paused = live.status == RideStatus.PAUSED
    val s = live.stats

    // Gauge scale: at least 60 km/h (40 mph), growing in steps of 20 (10) when you go faster.
    val minScale = if (unit == SpeedUnit.KMH) 60f else 40f
    val stepScale = if (unit == SpeedUnit.KMH) 20f else 10f
    val topSpeedInUnit = Format.speedValue(max(live.currentSpeedMps, s.maxSpeedMps), unit)
    val scale = max(minScale, ceil(topSpeedInUnit / stepScale) * stepScale)

    Column(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(horizontal = 20.dp),
    ) {
        Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = onBack) { Text("‹ Rides") }
            Row(Modifier.weight(1f), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
                GpsChip(live.gps, live.accuracyM)
                IconButton(onClick = onBigMode) { Icon(Icons.Filled.Fullscreen, "Big speed mode") }
            }
        }

        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            SpeedGauge(
                value = Format.speedValue(live.currentSpeedMps, unit),
                max = scale,
                unitLabel = if (paused) "PAUSED" else unit.label,
                valueText = Format.speed(live.currentSpeedMps, unit),
                modifier = Modifier.padding(horizontal = 12.dp),
            )
            StatGrid(
                listOf(
                    Triple("Max speed", Format.speed(s.maxSpeedMps, unit), unit.label),
                    Triple("Distance", Format.distance(s.distanceM, unit), unit.distanceLabel),
                    Triple("Time", Format.duration(s.elapsedMs), ""),
                    Triple("Avg moving", Format.speed(s.avgMovingSpeedMps, unit, 1), unit.label),
                    Triple("Avg overall", Format.speed(s.avgSpeedMps, unit, 1), unit.label),
                    Triple("Climb", Format.metres(s.elevationGainM), "m"),
                ),
            )
            if (live.route.size >= 2) {
                SurfaceCard(Modifier.fillMaxWidth()) {
                    RoutePreview(live.route, Orange, Modifier.fillMaxWidth().height(150.dp).padding(8.dp), strokeWidth = 4.dp)
                }
            }
        }

        Row(Modifier.fillMaxWidth().padding(vertical = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            FilledTonalButton(
                onClick = onPauseResume,
                modifier = Modifier.weight(1f).height(60.dp),
                shape = RoundedCornerShape(30.dp),
            ) {
                Icon(if (paused) Icons.Filled.PlayArrow else Icons.Filled.Pause, null)
                Text(if (paused) "  Resume" else "  Pause", style = MaterialTheme.typography.titleMedium)
            }
            Button(
                onClick = { confirmStop = true },
                modifier = Modifier.weight(1f).height(60.dp),
                shape = RoundedCornerShape(30.dp),
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error, contentColor = MaterialTheme.colorScheme.onError),
            ) {
                Icon(Icons.Filled.Stop, null)
                Text("  End ride", style = MaterialTheme.typography.titleMedium)
            }
        }
    }

    if (confirmStop) {
        AlertDialog(
            onDismissRequest = { confirmStop = false },
            title = { Text("End this ride?") },
            text = { Text("It will be saved with all its stats. You can find it in your rides list.") },
            confirmButton = { TextButton(onClick = { confirmStop = false; onStop() }) { Text("End ride") } },
            dismissButton = { TextButton(onClick = { confirmStop = false }) { Text("Keep riding") } },
        )
    }
}
