package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CompareArrows
import androidx.compose.material.icons.filled.EmojiEvents
import androidx.compose.material.icons.filled.FiberManualRecord
import androidx.compose.material.icons.filled.LocalGasStation
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.filled.TwoWheeler
import androidx.compose.material3.AssistChip
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStatus
import com.mopedtracker.data.RouteCodec
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.RoutePreview
import com.mopedtracker.ui.components.StatTile
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.WarnAmber

@Composable
fun HomeScreen(
    sessions: List<SessionEntity>,
    live: LiveState,
    unit: SpeedUnit,
    starredOnly: Boolean = false,
    onStart: () -> Unit,
    onOpenLive: () -> Unit,
    onOpenSession: (Long) -> Unit,
    onToggleUnit: () -> Unit,
    onToggleStarred: () -> Unit = {},
    onOpenRecords: () -> Unit = {},
    onOpenFuel: () -> Unit = {},
    onOpenCompare: () -> Unit = {},
) {
    val active = live.status != RideStatus.IDLE
    val shown = if (starredOnly) sessions.filter { it.starred } else sessions
    Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background)) {
        LazyColumn(
            Modifier.fillMaxSize().statusBarsPadding(),
            contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 120.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            item {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Moped Tracker", style = MaterialTheme.typography.headlineLarge)
                        Text("Your rides", style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    TextButton(onClick = onToggleUnit) { Text(unit.label, fontWeight = FontWeight.Bold) }
                }
            }
            item {
                Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = starredOnly, onClick = onToggleStarred, label = { Text("Starred") },
                        leadingIcon = { Icon(if (starredOnly) Icons.Filled.Star else Icons.Filled.StarBorder, null, Modifier.size(18.dp)) },
                    )
                    AssistChip(onClick = onOpenRecords, label = { Text("Records") }, leadingIcon = { Icon(Icons.Filled.EmojiEvents, null, Modifier.size(18.dp)) })
                    AssistChip(onClick = onOpenFuel, label = { Text("Fuel") }, leadingIcon = { Icon(Icons.Filled.LocalGasStation, null, Modifier.size(18.dp)) })
                    AssistChip(onClick = onOpenCompare, label = { Text("Compare") }, leadingIcon = { Icon(Icons.Filled.CompareArrows, null, Modifier.size(18.dp)) })
                }
            }
            if (active) item { ActiveBanner(live, unit, onOpenLive) }
            if (sessions.isNotEmpty()) item { Totals(sessions, unit) }
            if (sessions.isEmpty() && !active) item { EmptyState() }
            if (starredOnly && shown.isEmpty() && sessions.isNotEmpty()) {
                item { Text("No starred rides yet. Open a ride and tap the star.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(shown, key = { it.id }) { SessionCard(it, unit) { onOpenSession(it.id) } }
        }
        ExtendedFloatingActionButton(
            onClick = if (active) onOpenLive else onStart,
            modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding().padding(24.dp).height(60.dp),
            containerColor = Orange,
            contentColor = MaterialTheme.colorScheme.onPrimary,
            shape = RoundedCornerShape(30.dp),
        ) {
            Icon(if (active) Icons.Filled.FiberManualRecord else Icons.Filled.PlayArrow, null)
            Text(
                if (active) "  Ride in progress" else "  Start ride",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(end = 8.dp),
            )
        }
    }
}

@Composable
private fun ActiveBanner(live: LiveState, unit: SpeedUnit, onClick: () -> Unit) {
    SurfaceCard(Modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(Modifier.padding(18.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Filled.FiberManualRecord, null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(18.dp))
            Column(Modifier.padding(start = 12.dp).weight(1f)) {
                Text(
                    if (live.status == RideStatus.PAUSED) "Ride paused" else "Recording…",
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    "${Format.distance(live.stats.distanceM, unit)} ${unit.distanceLabel} · ${Format.duration(live.stats.elapsedMs)}",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text("${Format.speed(live.currentSpeedMps, unit)} ${unit.label}", style = MaterialTheme.typography.titleLarge, color = Orange)
        }
    }
}

@Composable
private fun Totals(sessions: List<SessionEntity>, unit: SpeedUnit) {
    val distance = sessions.sumOf { it.distanceM }
    val top = sessions.maxOf { it.maxSpeedMps }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        StatTile("Rides", "${sessions.size}", "", Modifier.weight(1f))
        StatTile("Total", Format.distance(distance, unit), unit.distanceLabel, Modifier.weight(1.4f))
        StatTile("Top", Format.speed(top, unit), unit.label, Modifier.weight(1.3f), accent = Orange)
    }
}

@Composable
private fun SessionCard(s: SessionEntity, unit: SpeedUnit, onClick: () -> Unit) {
    SurfaceCard(Modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(84.dp), contentAlignment = Alignment.Center) {
                RoutePreview(RouteCodec.decode(s.previewRoute), Orange, Modifier.fillMaxSize())
            }
            Column(Modifier.padding(start = 16.dp).weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(s.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f, fill = false), maxLines = 1)
                    if (s.starred) Icon(Icons.Filled.Star, "Starred", tint = WarnAmber, modifier = Modifier.padding(start = 6.dp).size(18.dp))
                }
                Text(Format.dateTime(s.startTime), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Mini("${Format.distance(s.distanceM, unit)} ${unit.distanceLabel}")
                    Mini(Format.duration(s.elapsedMs))
                }
                Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Mini("max ${Format.speed(s.maxSpeedMps, unit)}", Orange)
                    Mini("avg ${Format.speed(s.avgMovingSpeedMps, unit)} ${unit.label}")
                }
            }
        }
    }
}

@Composable
private fun Mini(text: String, color: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.onSurface) {
    Text(text, style = MaterialTheme.typography.labelLarge, color = color)
}

@Composable
private fun EmptyState() {
    Column(
        Modifier.fillMaxWidth().padding(top = 80.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(Icons.Filled.TwoWheeler, null, Modifier.size(72.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("No rides yet", style = MaterialTheme.typography.titleLarge)
        Text(
            "Tap Start ride, then go for a spin.\nYour speed, distance and route are saved here.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center,
        )
    }
}
