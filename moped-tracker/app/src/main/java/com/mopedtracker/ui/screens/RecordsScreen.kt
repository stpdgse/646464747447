package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.EmojiEvents
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.RecordKind
import com.mopedtracker.core.RideRecord
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange
import com.mopedtracker.ui.theme.WarnAmber

fun recordValueText(kind: RecordKind, value: Double, unit: SpeedUnit): String = when (kind) {
    RecordKind.TOP_SPEED, RecordKind.FASTEST_AVERAGE -> "${Format.speed(value.toFloat(), unit, if (kind == RecordKind.FASTEST_AVERAGE) 1 else 0)} ${unit.label}"
    RecordKind.LONGEST_DISTANCE -> "${Format.distance(value, unit)} ${unit.distanceLabel}"
    RecordKind.LONGEST_DURATION -> Format.duration(value.toLong())
    RecordKind.MOST_CLIMB -> "${Format.metres(value)} m"
}

@Composable
fun RecordsScreen(
    records: List<RideRecord>,
    starred: List<SessionEntity>,
    unit: SpeedUnit,
    onBack: () -> Unit,
    onOpen: (Long) -> Unit,
) {
    Column(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).statusBarsPadding()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
            Text("Records & favourites", style = MaterialTheme.typography.titleLarge)
        }
        LazyColumn(
            Modifier.fillMaxSize().navigationBarsPadding(),
            contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (records.isEmpty()) {
                item { Text("Your personal bests will appear here after your first ride.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(records, key = { it.kind }) { r ->
                SurfaceCard(Modifier.fillMaxWidth().clickable { onOpen(r.session.id) }) {
                    Row(Modifier.padding(18.dp), verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Filled.EmojiEvents, null, tint = WarnAmber, modifier = Modifier.size(34.dp))
                        Column(Modifier.padding(start = 16.dp).weight(1f)) {
                            Text(r.kind.title.uppercase(), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(recordValueText(r.kind, r.value, unit), style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold, color = Orange)
                            Text("${r.session.name} · ${Format.date(r.session.startTime)}", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
            item {
                Row(Modifier.padding(top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Filled.Star, null, tint = WarnAmber, modifier = Modifier.size(22.dp))
                    Text("  Starred rides", style = MaterialTheme.typography.titleMedium)
                }
            }
            if (starred.isEmpty()) {
                item { Text("Tap the star on a ride to keep your favourites here.", color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
            items(starred, key = { "s${it.id}" }) { s ->
                SurfaceCard(Modifier.fillMaxWidth().clickable { onOpen(s.id) }) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(s.name, style = MaterialTheme.typography.titleMedium)
                        Text(
                            "${Format.date(s.startTime)} · ${Format.distance(s.distanceM, unit)} ${unit.distanceLabel} · max ${Format.speed(s.maxSpeedMps, unit)} ${unit.label}",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}
