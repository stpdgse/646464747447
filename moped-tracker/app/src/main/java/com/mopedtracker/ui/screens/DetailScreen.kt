package com.mopedtracker.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.background
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import com.mopedtracker.core.Analysis
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.RouteMap
import com.mopedtracker.ui.components.RoutePoint
import com.mopedtracker.ui.components.SpeedChart
import com.mopedtracker.ui.components.SpeedHistogram
import com.mopedtracker.ui.components.StatGrid
import com.mopedtracker.ui.components.StatTile
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.speedColor

sealed interface DetailState {
    data object Loading : DetailState
    data object NotFound : DetailState
    data class Ready(val session: SessionEntity, val points: List<TrackPointEntity>) : DetailState
}

@Composable
fun DetailScreen(
    state: DetailState,
    unit: SpeedUnit,
    onBack: () -> Unit,
    onRename: (String) -> Unit,
    onDelete: () -> Unit,
) {
    Column(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).statusBarsPadding()) {
        when (state) {
            DetailState.Loading -> Box(Modifier.fillMaxSize(), Alignment.Center) { CircularProgressIndicator() }
            DetailState.NotFound -> Box(Modifier.fillMaxSize(), Alignment.Center) { Text("This ride no longer exists") }
            is DetailState.Ready -> ReadyContent(state, unit, onBack, onRename, onDelete)
        }
    }
}

@Composable
private fun ReadyContent(
    state: DetailState.Ready,
    unit: SpeedUnit,
    onBack: () -> Unit,
    onRename: (String) -> Unit,
    onDelete: () -> Unit,
) {
    val s = state.session
    var renaming by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf(false) }

    val routePoints = remember(state.points) {
        Analysis.thin(state.points, 600).map { RoutePoint(it.lat, it.lon, it.speedMps) }
    }
    val chartValues = remember(state.points, unit) {
        Analysis.thin(state.points, 240).map { Format.speedValue(it.speedMps, unit) }
    }
    val histogram = remember(state.points, unit) { Analysis.speedHistogram(state.points, unit.perMps, unit.histogramStep) }

    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
        Column(Modifier.weight(1f)) {
            Text(s.name, style = MaterialTheme.typography.titleLarge, maxLines = 1)
            Text(Format.dateTime(s.startTime), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        IconButton(onClick = { renaming = true }) { Icon(Icons.Filled.Edit, "Rename") }
        IconButton(onClick = { deleting = true }) { Icon(Icons.Filled.Delete, "Delete", tint = MaterialTheme.colorScheme.error) }
    }

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            SurfaceCard(Modifier.fillMaxWidth()) {
                Column {
                    RouteMap(routePoints, s.maxSpeedMps, Modifier.fillMaxWidth().height(280.dp))
                    SpeedLegend(unit, s.maxSpeedMps)
                }
            }
        }
        item {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                StatTile("Distance", Format.distance(s.distanceM, unit), unit.distanceLabel, Modifier.weight(1f))
                StatTile("Max speed", Format.speed(s.maxSpeedMps, unit), unit.label, Modifier.weight(1f), accent = Orange)
            }
        }
        item {
            StatGrid(
                listOf(
                    Triple("Duration", Format.duration(s.elapsedMs), ""),
                    Triple("Moving time", Format.duration(s.movingMs), ""),
                    Triple("Avg moving speed", Format.speed(s.avgMovingSpeedMps, unit, 1), unit.label),
                    Triple("Avg overall speed", Format.speed(s.avgSpeedMps, unit, 1), unit.label),
                    Triple("Stopped", Format.duration((s.elapsedMs - s.movingMs).coerceAtLeast(0)), ""),
                    Triple("Climb / descent", "+${Format.metres(s.elevationGainM)} / -${Format.metres(s.elevationLossM)}", "m"),
                    Triple("Highest point", s.maxAltitudeM?.let { Format.metres(it) } ?: "–", "m"),
                    Triple("Lowest point", s.minAltitudeM?.let { Format.metres(it) } ?: "–", "m"),
                    Triple("Started", Format.time(s.startTime), ""),
                    Triple("Ended", s.endTime?.let { Format.time(it) } ?: "–", ""),
                    Triple("GPS points", "${s.pointCount}", ""),
                    Triple("Avg GPS accuracy", "±${Format.metres(s.avgAccuracyM.toDouble())}", "m"),
                ),
            )
        }
        item {
            SurfaceCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("Speed over time", style = MaterialTheme.typography.titleMedium)
                    SpeedChart(chartValues, unit.label, Format.duration(s.elapsedMs))
                }
            }
        }
        if (histogram.isNotEmpty()) {
            item {
                SurfaceCard(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text("Speed distribution", style = MaterialTheme.typography.titleMedium)
                        SpeedHistogram(histogram, unit.label)
                    }
                }
            }
        }
        item { Box(Modifier.navigationBarsPadding()) }
    }

    if (renaming) {
        var text by remember { mutableStateOf(s.name) }
        AlertDialog(
            onDismissRequest = { renaming = false },
            title = { Text("Rename ride") },
            text = { OutlinedTextField(text, { text = it }, singleLine = true) },
            confirmButton = { TextButton(onClick = { renaming = false; onRename(text) }) { Text("Save") } },
            dismissButton = { TextButton(onClick = { renaming = false }) { Text("Cancel") } },
        )
    }
    if (deleting) {
        AlertDialog(
            onDismissRequest = { deleting = false },
            title = { Text("Delete this ride?") },
            text = { Text("The ride and its route can't be recovered.") },
            confirmButton = {
                TextButton(onClick = { deleting = false; onDelete() }) { Text("Delete", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { deleting = false }) { Text("Cancel") } },
        )
    }
}

/** Colour ramp legend for the route map. */
@Composable
private fun SpeedLegend(unit: SpeedUnit, maxSpeedMps: Float) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("0", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(Modifier.weight(1f).padding(horizontal = 8.dp).height(8.dp)) {
            val steps = 24
            repeat(steps) { Box(Modifier.weight(1f).fillMaxSize().background(speedColor(it / (steps - 1f)))) }
        }
        Text("${Format.speed(maxSpeedMps, unit)} ${unit.label}", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
