package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.Analysis
import com.mopedtracker.core.EventType
import com.mopedtracker.core.Events
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.CompareChart
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.Teal

private val ColorA = Orange
private val ColorB = Teal

private data class Row3(val label: String, val a: String, val b: String, val aValue: Double, val bValue: Double)

@Composable
fun CompareScreen(
    sessions: List<SessionEntity>,
    a: SessionEntity?,
    b: SessionEntity?,
    pointsA: List<TrackPointEntity>,
    pointsB: List<TrackPointEntity>,
    unit: SpeedUnit,
    onPickA: (Long) -> Unit,
    onPickB: (Long) -> Unit,
    onBack: () -> Unit,
) {
    Column(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).statusBarsPadding()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
            Text("Compare rides", style = MaterialTheme.typography.titleLarge)
        }
        if (sessions.size < 2) {
            Box(Modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
                Text("Record at least two rides to compare them.", textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            return@Column
        }
        LazyColumn(
            Modifier.fillMaxSize().navigationBarsPadding(),
            contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Picker("A", ColorA, sessions, a, onPickA, Modifier.weight(1f))
                    Picker("B", ColorB, sessions, b, onPickB, Modifier.weight(1f))
                }
            }
            if (a != null && b != null) {
                val rows = buildRows(a, b, pointsA, pointsB, unit)
                item {
                    SurfaceCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(vertical = 8.dp)) {
                            rows.forEachIndexed { i, r ->
                                if (i > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f))
                                Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                                    Text(r.label, Modifier.weight(1.2f), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Value(r.a, r.aValue > r.bValue, ColorA, Modifier.weight(1f))
                                    Value(r.b, r.bValue > r.aValue, ColorB, Modifier.weight(1f))
                                }
                            }
                        }
                    }
                }
                item {
                    SurfaceCard(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text("Speed over time", style = MaterialTheme.typography.titleMedium)
                            CompareChart(
                                Analysis.speedSeries(pointsA, unit.perMps), Analysis.speedSeries(pointsB, unit.perMps),
                                ColorA, ColorB, unit.label,
                            )
                            Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                                Legend("A · ${a.name}", ColorA)
                                Legend("B · ${b.name}", ColorB)
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun buildRows(a: SessionEntity, b: SessionEntity, pa: List<TrackPointEntity>, pb: List<TrackPointEntity>, unit: SpeedUnit): List<Row3> {
    val ea = Events.detect(pa)
    val eb = Events.detect(pb)
    fun speed(mps: Float, d: Int = 0) = "${Format.speed(mps, unit, d)} ${unit.label}"
    fun dist(m: Double) = "${Format.distance(m, unit)} ${unit.distanceLabel}"
    return listOf(
        Row3("Distance", dist(a.distanceM), dist(b.distanceM), a.distanceM, b.distanceM),
        Row3("Duration", Format.duration(a.elapsedMs), Format.duration(b.elapsedMs), a.elapsedMs.toDouble(), b.elapsedMs.toDouble()),
        Row3("Moving time", Format.duration(a.movingMs), Format.duration(b.movingMs), a.movingMs.toDouble(), b.movingMs.toDouble()),
        Row3("Max speed", speed(a.maxSpeedMps), speed(b.maxSpeedMps), a.maxSpeedMps.toDouble(), b.maxSpeedMps.toDouble()),
        Row3("Avg moving speed", speed(a.avgMovingSpeedMps, 1), speed(b.avgMovingSpeedMps, 1), a.avgMovingSpeedMps.toDouble(), b.avgMovingSpeedMps.toDouble()),
        Row3("Avg overall speed", speed(a.avgSpeedMps, 1), speed(b.avgSpeedMps, 1), a.avgSpeedMps.toDouble(), b.avgSpeedMps.toDouble()),
        Row3("Climb", "${Format.metres(a.elevationGainM)} m", "${Format.metres(b.elevationGainM)} m", a.elevationGainM, b.elevationGainM),
        Row3("Hard braking", "${ea.count { it.type == EventType.HARD_BRAKE }}", "${eb.count { it.type == EventType.HARD_BRAKE }}",
            ea.count { it.type == EventType.HARD_BRAKE }.toDouble(), eb.count { it.type == EventType.HARD_BRAKE }.toDouble()),
        Row3("Hard acceleration", "${ea.count { it.type == EventType.HARD_ACCEL }}", "${eb.count { it.type == EventType.HARD_ACCEL }}",
            ea.count { it.type == EventType.HARD_ACCEL }.toDouble(), eb.count { it.type == EventType.HARD_ACCEL }.toDouble()),
    )
}

@Composable
private fun Value(text: String, higher: Boolean, color: Color, modifier: Modifier) {
    Text(
        text, modifier, textAlign = TextAlign.End,
        style = MaterialTheme.typography.bodyLarge,
        fontWeight = if (higher) FontWeight.Bold else FontWeight.Normal,
        color = if (higher) color else MaterialTheme.colorScheme.onSurface,
    )
}

@Composable
private fun Legend(text: String, color: Color) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(10.dp).background(color, CircleShape))
        Text("  $text", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
    }
}

@Composable
private fun Picker(tag: String, color: Color, sessions: List<SessionEntity>, selected: SessionEntity?, onPick: (Long) -> Unit, modifier: Modifier) {
    var open by remember { mutableStateOf(false) }
    Box(modifier) {
        SurfaceCard(Modifier.fillMaxWidth().clickable { open = true }) {
            Row(Modifier.padding(horizontal = 14.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(26.dp).background(color, CircleShape), contentAlignment = Alignment.Center) {
                    Text(tag, color = Color.Black, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.Bold)
                }
                Column(Modifier.padding(start = 10.dp).weight(1f)) {
                    Text(selected?.name ?: "Choose ride", style = MaterialTheme.typography.titleMedium, maxLines = 1)
                    Text(selected?.let { Format.date(it.startTime) } ?: "", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Icon(Icons.Filled.ArrowDropDown, null)
            }
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            sessions.forEach { s ->
                DropdownMenuItem(
                    text = { Text("${s.name} · ${Format.date(s.startTime)}") },
                    onClick = { open = false; onPick(s.id) },
                )
            }
        }
    }
}
