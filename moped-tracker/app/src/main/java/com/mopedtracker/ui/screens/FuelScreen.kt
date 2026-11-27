package com.mopedtracker.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.mopedtracker.core.FuelCalc
import com.mopedtracker.core.FuelStats
import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.components.StatTile
import com.mopedtracker.ui.components.SurfaceCard
import com.mopedtracker.ui.theme.Orange

@Composable
fun FuelScreen(
    entries: List<FuelEntryEntity>,
    stats: FuelStats,
    unit: SpeedUnit,
    onBack: () -> Unit,
    onAdd: (FuelEntryEntity) -> Unit,
    onDelete: (Long) -> Unit,
) {
    var adding by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf<FuelEntryEntity?>(null) }

    Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background)) {
        Column(Modifier.fillMaxSize().statusBarsPadding()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
                Column(Modifier.weight(1f)) {
                    Text("Fuel & costs", style = MaterialTheme.typography.titleLarge)
                    Text("${stats.fillUps} fill-ups", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            LazyColumn(
                Modifier.fillMaxSize(),
                contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 120.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                item { Summary(stats, unit) }
                if (entries.isEmpty()) item { Hint("Log your first fill-up with the button below. Fill to the brim each time (tick \"Full tank\") and the app works out your consumption and cost per km from your recorded rides.") }
                else if (stats.consumptionLPer100Km == null) item { Hint("Consumption appears once you have two full-tank fill-ups with recorded rides in between.") }
                items(entries, key = { it.id }) { e -> EntryRow(e, onDelete = { deleting = e }) }
            }
        }
        ExtendedFloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding().padding(24.dp),
            containerColor = Orange,
            contentColor = MaterialTheme.colorScheme.onPrimary,
            shape = RoundedCornerShape(30.dp),
        ) {
            Icon(Icons.Filled.Add, null)
            Text("  Add fill-up", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(end = 8.dp))
        }
    }

    if (adding) AddFuelDialog(onDismiss = { adding = false }, onSave = { adding = false; onAdd(it) })
    deleting?.let { e ->
        AlertDialog(
            onDismissRequest = { deleting = null },
            title = { Text("Delete this fill-up?") },
            text = { Text("${Format.liters(e.liters)} L · ${Format.money(e.totalCost)} on ${Format.date(e.time)}") },
            confirmButton = { TextButton(onClick = { deleting = null; onDelete(e.id) }) { Text("Delete", color = MaterialTheme.colorScheme.error) } },
            dismissButton = { TextButton(onClick = { deleting = null }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun Summary(stats: FuelStats, unit: SpeedUnit) {
    val perDistance = if (unit == SpeedUnit.KMH) "km" else "mi"
    val consumption = stats.consumptionLPer100Km?.let {
        String.format(java.util.Locale.US, "%.1f", if (unit == SpeedUnit.KMH) it else FuelCalc.mpg(it))
    } ?: "–"
    val consumptionUnit = if (unit == SpeedUnit.KMH) "L/100 km" else "mpg"
    val costPer = stats.costPerKm?.let { Format.money(if (unit == SpeedUnit.KMH) it else it * 1.609344) } ?: "–"
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            StatTile("Total spent", Format.money(stats.totalCost), "", Modifier.weight(1f), accent = Orange)
            StatTile("Fuel bought", Format.liters(stats.totalLiters), "L", Modifier.weight(1f))
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            StatTile("Consumption", consumption, consumptionUnit, Modifier.weight(1f))
            StatTile("Cost per $perDistance", costPer, "", Modifier.weight(1f))
        }
        StatTile("Avg price per litre", Format.money(stats.avgPricePerLiter), "", Modifier.fillMaxWidth())
    }
}

@Composable
private fun Hint(text: String) {
    Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(vertical = 4.dp))
}

@Composable
private fun EntryRow(e: FuelEntryEntity, onDelete: () -> Unit) {
    SurfaceCard(Modifier.fillMaxWidth()) {
        Row(Modifier.padding(start = 18.dp, top = 12.dp, bottom = 12.dp, end = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(Format.date(e.time), style = MaterialTheme.typography.titleMedium)
                Text(
                    "${Format.liters(e.liters)} L · ${Format.money(e.totalCost)} · ${Format.money(if (e.liters > 0) e.totalCost / e.liters else 0.0)}/L",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (e.fullTank || e.note.isNotBlank()) {
                    Text(
                        listOfNotNull("Full tank".takeIf { e.fullTank }, e.note.takeIf { it.isNotBlank() }).joinToString(" · "),
                        style = MaterialTheme.typography.labelMedium,
                        color = Orange,
                    )
                }
            }
            IconButton(onClick = onDelete) { Icon(Icons.Filled.Delete, "Delete", tint = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
    }
}

/** Accepts "3,45" and "3.45" so it works with any keyboard locale. */
private fun parseNumber(s: String): Double? = s.trim().replace(',', '.').toDoubleOrNull()?.takeIf { it > 0 && it.isFinite() }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AddFuelDialog(onDismiss: () -> Unit, onSave: (FuelEntryEntity) -> Unit) {
    var liters by remember { mutableStateOf("") }
    var cost by remember { mutableStateOf("") }
    var full by remember { mutableStateOf(true) }
    var note by remember { mutableStateOf("") }
    var pickingDate by remember { mutableStateOf(false) }
    val dateState = rememberDatePickerState(initialSelectedDateMillis = System.currentTimeMillis())
    val time = dateState.selectedDateMillis ?: System.currentTimeMillis()

    val l = parseNumber(liters)
    val c = parseNumber(cost)
    val perLiter = if (l != null && c != null) c / l else null

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add fill-up") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                TextButton(onClick = { pickingDate = true }) { Text("Date: ${Format.date(time)}") }
                OutlinedTextField(liters, { liters = it }, label = { Text("Litres") }, singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal))
                OutlinedTextField(cost, { cost = it }, label = { Text("Total cost") }, singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    supportingText = { perLiter?.let { Text("${Format.money(it)} per litre") } })
                OutlinedTextField(note, { note = it }, label = { Text("Note (optional)") }, singleLine = true)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(full, { full = it })
                    Text("Full tank (filled to the brim)")
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = l != null && c != null,
                onClick = { onSave(FuelEntryEntity(time = time, liters = l!!, totalCost = c!!, fullTank = full, note = note.trim())) },
            ) { Text("Save") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
    if (pickingDate) {
        DatePickerDialog(
            onDismissRequest = { pickingDate = false },
            confirmButton = { TextButton(onClick = { pickingDate = false }) { Text("OK") } },
        ) { DatePicker(state = dateState) }
    }
}
