package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.Analysis
import com.mopedtracker.core.EventType
import com.mopedtracker.core.Events
import com.mopedtracker.core.RideEvent
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.core.VehicleStatsCalc
import com.mopedtracker.ui.components.VehiclePhoto
import com.mopedtracker.ui.components.specParts
import com.mopedtracker.ui.components.subtitle
import androidx.compose.ui.draw.clip
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.RouteMap
import com.mopedtracker.ui.components.RoutePoint
import com.mopedtracker.ui.components.SpeedChart
import com.mopedtracker.ui.components.SpeedHistogram
import com.mopedtracker.ui.components.StatGrid
import com.mopedtracker.ui.components.StatTile
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.BadRed
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.Teal
import com.mopedtracker.ui.theme.WarnAmber
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
    onToggleStar: () -> Unit = {},
    onShare: (SessionEntity, List<TrackPointEntity>) -> Unit = { _, _ -> },
    vehicle: VehicleEntity? = null,
) {
    Column(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).statusBarsPadding()) {
        when (state) {
            DetailState.Loading -> Box(Modifier.fillMaxSize(), Alignment.Center) { CircularProgressIndicator() }
            DetailState.NotFound -> Box(Modifier.fillMaxSize(), Alignment.Center) { Text("This ride no longer exists") }
            is DetailState.Ready -> ReadyContent(state, unit, onBack, onRename, onDelete, onToggleStar, onShare, vehicle)
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
    onToggleStar: () -> Unit,
    onShare: (SessionEntity, List<TrackPointEntity>) -> Unit,
    vehicle: VehicleEntity?,
) {
    val s = state.session
    var renaming by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf(false) }
    var menu by remember { mutableStateOf(false) }

    val routePoints = remember(state.points) {
        Analysis.thin(state.points, 600).map { RoutePoint(it.lat, it.lon, it.speedMps) }
    }
    val chartValues = remember(state.points, unit) {
        Analysis.thin(state.points, 240).map { Format.speedValue(it.speedMps, unit) }
    }
    val histogram = remember(state.points, unit) { Analysis.speedHistogram(state.points, unit.perMps, unit.histogramStep) }
    val events = remember(state.points) { Events.detect(state.points) }

    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
        Column(Modifier.weight(1f)) {
            Text(s.name, style = MaterialTheme.typography.titleLarge, maxLines = 1)
            Text(Format.dateTime(s.startTime), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        IconButton(onClick = onToggleStar) {
            Icon(
                if (s.starred) Icons.Filled.Star else Icons.Filled.StarBorder,
                if (s.starred) "Remove star" else "Star this ride",
                tint = if (s.starred) WarnAmber else MaterialTheme.colorScheme.onSurface,
            )
        }
        IconButton(onClick = { onShare(s, state.points) }) { Icon(Icons.Filled.Share, "Share as image") }
        Box {
            IconButton(onClick = { menu = true }) { Icon(Icons.Filled.MoreVert, "More") }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                DropdownMenuItem(text = { Text("Rename") }, onClick = { menu = false; renaming = true })
                DropdownMenuItem(text = { Text("Delete", color = MaterialTheme.colorScheme.error) }, onClick = { menu = false; deleting = true })
            }
        }
    }

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            SurfaceCard(Modifier.fillMaxWidth()) {
                Column {
                    RouteMap(routePoints, s.maxSpeedMps, Modifier.fillMaxWidth().height(280.dp), events)
                    SpeedLegend(unit, s.maxSpeedMps)
                }
            }
        }
        if (vehicle != null) item { VehicleCard(vehicle, s.maxSpeedMps) }
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
        item { EventsCard(events, unit) }
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

@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
private fun VehicleCard(v: VehicleEntity, maxSpeedMps: Float) {
    val share = VehicleStatsCalc.shareOfClaimedTop(maxSpeedMps, v.claimedTopKmh)
    SurfaceCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                VehiclePhoto(v.photoPath, Modifier.size(64.dp).clip(androidx.compose.foundation.shape.RoundedCornerShape(16.dp)), maxSide = 240)
                Column(Modifier.padding(start = 14.dp)) {
                    Text(v.name, style = MaterialTheme.typography.titleMedium)
                    Text(v.subtitle(), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            val specs = v.specParts()
            if (specs.isNotEmpty()) {
                androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    specs.forEach { SpecPill(it) }
                }
            }
            if (share != null) {
                Text(
                    "Reached ${(share * 100).toInt()}% of the claimed ${Format.number(v.claimedTopKmh!!)} km/h top speed.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun EventsCard(events: List<RideEvent>, unit: SpeedUnit) {
    SurfaceCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                Text("Events", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                Dot(BadRed, "${events.count { it.type == EventType.HARD_BRAKE }} braking")
                Dot(Teal, "${events.count { it.type == EventType.HARD_ACCEL }} launches")
            }
            if (events.isEmpty()) {
                Text(
                    "No hard braking or hard acceleration detected. A smooth ride.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            events.forEach { e ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.size(12.dp).background(if (e.type == EventType.HARD_BRAKE) BadRed else Teal, androidx.compose.foundation.shape.CircleShape))
                    Column(Modifier.padding(start = 12.dp).weight(1f)) {
                        Text(e.type.label, style = MaterialTheme.typography.bodyLarge)
                        Text(
                            "at ${Format.offset(e.offsetMs)} · ${Format.speed(e.fromMps, unit)} → ${Format.speed(e.toMps, unit)} ${unit.label}",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text(String.format(java.util.Locale.US, "%.1f m/s²", e.accelMps2), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

@Composable
private fun Dot(color: Color, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(10.dp).background(color, androidx.compose.foundation.shape.CircleShape))
        Text("  $label", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
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
