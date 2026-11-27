package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.TwoWheeler
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.VehicleStats
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.components.VehiclePhoto
import com.mopedtracker.ui.components.specParts
import com.mopedtracker.ui.components.subtitle
import com.mopedtracker.ui.theme.Orange

@Composable
fun GarageScreen(
    vehicles: List<VehicleEntity>,
    stats: Map<Long?, VehicleStats>,
    unit: SpeedUnit,
    onBack: () -> Unit,
    onAdd: () -> Unit,
    onOpen: (Long) -> Unit,
) {
    Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background)) {
        Column(Modifier.fillMaxSize().statusBarsPadding()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
                Column(Modifier.weight(1f)) {
                    Text("Garage", style = MaterialTheme.typography.titleLarge)
                    Text(
                        if (vehicles.size == 1) "1 vehicle" else "${vehicles.size} vehicles",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            LazyColumn(
                Modifier.fillMaxSize(),
                contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 120.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                if (vehicles.isEmpty()) item { EmptyGarage() }
                items(vehicles, key = { it.id }) { v -> VehicleCard(v, stats[v.id] ?: VehicleStats(), unit) { onOpen(v.id) } }
            }
        }
        ExtendedFloatingActionButton(
            onClick = onAdd,
            modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding().padding(24.dp),
            containerColor = Orange,
            contentColor = MaterialTheme.colorScheme.onPrimary,
            shape = RoundedCornerShape(30.dp),
        ) {
            Icon(Icons.Filled.Add, null)
            Text("  Add vehicle", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(end = 8.dp))
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun VehicleCard(v: VehicleEntity, stats: VehicleStats, unit: SpeedUnit, onClick: () -> Unit) {
    SurfaceCard(Modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Column {
            VehiclePhoto(v.photoPath, Modifier.fillMaxWidth().height(170.dp), maxSide = 900)
            Column(Modifier.padding(18.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Column {
                    Text(v.name, style = MaterialTheme.typography.titleLarge)
                    Text(v.subtitle(), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                val specs = v.specParts()
                if (specs.isNotEmpty()) {
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        specs.forEach { SpecPill(it) }
                    }
                }
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.25f))
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Lifetime("Rides", "${stats.rides}")
                    Lifetime("Distance", "${Format.distance(stats.distanceM, unit)} ${unit.distanceLabel}")
                    Lifetime("Top speed", if (stats.rides > 0) "${Format.speed(stats.topSpeedMps, unit)} ${unit.label}" else "–", highlight = true)
                }
            }
        }
    }
}

@Composable
fun SpecPill(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.onSurface,
        modifier = Modifier
            .background(MaterialTheme.colorScheme.surfaceVariant, RoundedCornerShape(50))
            .padding(horizontal = 12.dp, vertical = 6.dp),
    )
}

@Composable
private fun Lifetime(label: String, value: String, highlight: Boolean = false) {
    Column {
        Text(label.uppercase(), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = if (highlight) Orange else MaterialTheme.colorScheme.onSurface)
    }
}

@Composable
private fun EmptyGarage() {
    Column(
        Modifier.fillMaxWidth().padding(top = 72.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(Icons.Filled.TwoWheeler, null, Modifier.size(72.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("Your garage is empty", style = MaterialTheme.typography.titleLarge)
        Text(
            "Add your moped (or any vehicle) with a photo and its stats. You'll pick it each time you start a ride.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center,
        )
    }
}
