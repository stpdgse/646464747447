package com.mopedtracker.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.ui.components.VehiclePhoto
import com.mopedtracker.ui.components.specParts
import com.mopedtracker.ui.theme.Orange

/** "Which vehicle are you riding?" shown when a ride is started. [initialId] is preselected (the last one used). */
@Composable
fun VehiclePickerDialog(
    vehicles: List<VehicleEntity>,
    initialId: Long?,
    onStart: (vehicleId: Long?) -> Unit,
    onManage: () -> Unit,
    onDismiss: () -> Unit,
) {
    Dialog(onDismissRequest = onDismiss) {
        Card(
            shape = RoundedCornerShape(28.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        ) { VehiclePickerContent(vehicles, initialId, onStart, onManage, onDismiss) }
    }
}

/** The dialog body on its own, so it can also be rendered in screenshot tests. */
@Composable
fun VehiclePickerContent(
    vehicles: List<VehicleEntity>,
    initialId: Long?,
    onStart: (Long?) -> Unit,
    onManage: () -> Unit,
    onDismiss: () -> Unit,
) {
    var selected by remember { mutableStateOf(initialId?.takeIf { id -> vehicles.any { it.id == id } } ?: vehicles.firstOrNull()?.id) }
    Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("Which vehicle?", style = MaterialTheme.typography.titleLarge)
        Column(
            Modifier.heightIn(max = 380.dp).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            vehicles.forEach { v -> VehicleChoice(v, selected == v.id) { selected = v.id } }
            NoVehicleChoice(selected == null) { selected = null }
        }
        Button(
            onClick = { onStart(selected) },
            modifier = Modifier.fillMaxWidth().height(54.dp),
            shape = RoundedCornerShape(27.dp),
            colors = ButtonDefaults.buttonColors(containerColor = Orange, contentColor = MaterialTheme.colorScheme.onPrimary),
        ) {
            Icon(Icons.Filled.PlayArrow, null)
            Text("  Start ride", style = MaterialTheme.typography.titleMedium)
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            TextButton(onClick = onManage) { Text("Manage garage") }
            TextButton(onClick = onDismiss) { Text("Cancel") }
        }
    }
}

@Composable
private fun VehicleChoice(v: VehicleEntity, selected: Boolean, onClick: () -> Unit) {
    ChoiceRow(selected, onClick) {
        VehiclePhoto(v.photoPath, Modifier.size(60.dp).clip(RoundedCornerShape(14.dp)), maxSide = 200)
        Column(Modifier.padding(start = 14.dp).weight(1f)) {
            Text(v.name, style = MaterialTheme.typography.titleMedium, maxLines = 1)
            val specs = v.specParts().take(3).joinToString(" · ")
            Text(
                specs.ifEmpty { v.kind },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
            )
        }
    }
}

@Composable
private fun NoVehicleChoice(selected: Boolean, onClick: () -> Unit) {
    ChoiceRow(selected, onClick) {
        Text("No vehicle", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(vertical = 14.dp).weight(1f))
    }
}

@Composable
private fun ChoiceRow(selected: Boolean, onClick: () -> Unit, content: @Composable androidx.compose.foundation.layout.RowScope.() -> Unit) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = if (selected) 0.9f else 0.45f)),
        border = if (selected) BorderStroke(2.dp, Orange) else null,
    ) {
        Row(Modifier.fillMaxWidth().padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
            content()
            if (selected) Icon(Icons.Filled.CheckCircle, "Selected", tint = Orange, modifier = Modifier.padding(horizontal = 6.dp))
        }
    }
}
