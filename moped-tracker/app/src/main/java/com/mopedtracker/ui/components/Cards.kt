package com.mopedtracker.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.GpsQuality
import com.mopedtracker.ui.theme.BadRed
import com.mopedtracker.ui.theme.GoodGreen
import com.mopedtracker.ui.theme.WarnAmber

@Composable
fun SurfaceCard(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) { content() }
}

/** Label on top, big value, small unit. */
@Composable
fun StatTile(label: String, value: String, unit: String, modifier: Modifier = Modifier, accent: Color? = null) {
    Column(
        modifier
            .background(MaterialTheme.colorScheme.surface, RoundedCornerShape(20.dp))
            .padding(horizontal = 16.dp, vertical = 14.dp),
    ) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(
                value,
                style = MaterialTheme.typography.headlineMedium,
                color = accent ?: MaterialTheme.colorScheme.onSurface,
                fontWeight = FontWeight.Bold,
            )
            if (unit.isNotEmpty()) {
                Text(
                    unit,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 4.dp, bottom = 3.dp),
                )
            }
        }
    }
}

/** Rows of [columns] tiles. Tiles are (label, value, unit). */
@Composable
fun StatGrid(tiles: List<Triple<String, String, String>>, modifier: Modifier = Modifier, columns: Int = 2) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        tiles.chunked(columns).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                row.forEach { (l, v, u) -> StatTile(l, v, u, Modifier.weight(1f)) }
                repeat(columns - row.size) { Box(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
fun GpsChip(quality: GpsQuality, accuracyM: Float?, modifier: Modifier = Modifier) {
    val (color, text) = when (quality) {
        GpsQuality.SEARCHING -> WarnAmber to "Searching for GPS…"
        GpsQuality.WEAK -> BadRed to "GPS weak · ±${accuracyM?.toInt()} m"
        GpsQuality.OK -> WarnAmber to "GPS ok · ±${accuracyM?.toInt()} m"
        GpsQuality.GOOD -> GoodGreen to "GPS good · ±${accuracyM?.toInt()} m"
    }
    Row(
        modifier
            .background(MaterialTheme.colorScheme.surface, RoundedCornerShape(50))
            .padding(horizontal = 14.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(10.dp).background(color, CircleShape))
        Text(text, Modifier.padding(start = 8.dp), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurface)
    }
}
