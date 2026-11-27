package com.mopedtracker.ui.screens

import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CameraAlt
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.PhotoLibrary
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.components.VehiclePhoto
import com.mopedtracker.ui.theme.Orange
import kotlinx.coroutines.launch
import java.io.File

/** Everything the form needs from the outside world for photos, so the screen itself stays easy to test. */
class PhotoOps(
    val import: suspend (Uri) -> String?,
    val delete: (String?) -> Unit,
    val cameraTarget: () -> Pair<File, Uri>,
)

val VehicleKinds = listOf("Moped", "Scooter", "Motorcycle", "E-bike", "Other")

/** Blank is fine (null); anything else must be a non-negative number. Accepts "12,5" and "12.5". */
private fun parseNumber(s: String): Pair<Double?, Boolean> {
    val t = s.trim().replace(',', '.')
    if (t.isEmpty()) return null to true
    val d = t.toDoubleOrNull()
    return if (d != null && d.isFinite() && d >= 0) d to true else null to false
}

private fun parseYear(s: String): Pair<Int?, Boolean> {
    val t = s.trim()
    if (t.isEmpty()) return null to true
    val y = t.toIntOrNull()
    return if (y != null && y in 1900..2100) y to true else null to false
}

private fun initialText(v: Double?) = v?.let { Format.number(it) } ?: ""

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun VehicleEditScreen(
    initial: VehicleEntity?,
    photos: PhotoOps,
    onBack: () -> Unit,
    onSave: (VehicleEntity) -> Unit,
    onDelete: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    val originalPhoto = initial?.photoPath
    var photoPath by remember { mutableStateOf(originalPhoto) }
    var name by remember { mutableStateOf(initial?.name ?: "") }
    var kind by remember { mutableStateOf(initial?.kind ?: VehicleKinds.first()) }
    var hp by remember { mutableStateOf(initialText(initial?.hp)) }
    var nm by remember { mutableStateOf(initialText(initial?.torqueNm)) }
    var cc by remember { mutableStateOf(initialText(initial?.displacementCc)) }
    var kg by remember { mutableStateOf(initialText(initial?.weightKg)) }
    var year by remember { mutableStateOf(initial?.year?.toString() ?: "") }
    var tank by remember { mutableStateOf(initialText(initial?.tankLiters)) }
    var top by remember { mutableStateOf(initialText(initial?.claimedTopKmh)) }
    var notes by remember { mutableStateOf(initial?.notes ?: "") }
    var confirmDelete by remember { mutableStateOf(false) }

    /** Throws away a photo that was picked during this edit but never saved. */
    fun discardPendingPhoto() {
        if (photoPath != originalPhoto) photos.delete(photoPath)
    }

    fun adopt(path: String?) {
        if (path == null) {
            Toast.makeText(context, "Couldn't read that photo", Toast.LENGTH_SHORT).show()
            return
        }
        if (photoPath != originalPhoto) photos.delete(photoPath)
        photoPath = path
    }

    val gallery = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) scope.launch { adopt(photos.import(uri)) }
    }
    var cameraFile by remember { mutableStateOf<File?>(null) }
    var cameraUri by remember { mutableStateOf<Uri?>(null) }
    val camera = rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) { ok ->
        val uri = cameraUri
        val file = cameraFile
        if (ok && uri != null) scope.launch { adopt(photos.import(uri)); file?.delete() } else file?.delete()
    }

    val (hpV, hpOk) = parseNumber(hp)
    val (nmV, nmOk) = parseNumber(nm)
    val (ccV, ccOk) = parseNumber(cc)
    val (kgV, kgOk) = parseNumber(kg)
    val (yearV, yearOk) = parseYear(year)
    val (tankV, tankOk) = parseNumber(tank)
    val (topV, topOk) = parseNumber(top)
    val valid = name.isNotBlank() && hpOk && nmOk && ccOk && kgOk && yearOk && tankOk && topOk

    fun leave() {
        discardPendingPhoto()
        onBack()
    }
    BackHandler { leave() }

    Column(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).statusBarsPadding()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = ::leave) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
            Text(if (initial == null) "Add vehicle" else "Edit vehicle", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            if (initial != null) IconButton(onClick = { confirmDelete = true }) {
                Icon(Icons.Filled.Delete, "Delete vehicle", tint = MaterialTheme.colorScheme.error)
            }
        }

        Column(
            Modifier.weight(1f).imePadding().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            VehiclePhoto(photoPath, Modifier.fillMaxWidth().height(210.dp).clip(RoundedCornerShape(24.dp)), maxSide = 1024)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { gallery.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) }, Modifier.weight(1f)) {
                    Icon(Icons.Filled.PhotoLibrary, null)
                    Text("  Gallery")
                }
                OutlinedButton(
                    onClick = {
                        val (file, uri) = photos.cameraTarget()
                        cameraFile = file
                        cameraUri = uri
                        runCatching { camera.launch(uri) }.onFailure {
                            file.delete()
                            Toast.makeText(context, "No camera app available", Toast.LENGTH_SHORT).show()
                        }
                    },
                    Modifier.weight(1f),
                ) {
                    Icon(Icons.Filled.CameraAlt, null)
                    Text("  Camera")
                }
                if (photoPath != null) TextButton(onClick = {
                    if (photoPath != originalPhoto) photos.delete(photoPath)
                    photoPath = null
                }) { Text("Remove") }
            }

            OutlinedTextField(name, { name = it }, label = { Text("Name") }, singleLine = true, modifier = Modifier.fillMaxWidth(),
                supportingText = { if (name.isBlank()) Text("Required, e.g. \"Vespa GTS 125\"") })

            Text("Type", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                VehicleKinds.forEach { k -> FilterChip(selected = kind == k, onClick = { kind = k }, label = { Text(k) }) }
            }

            Text("Stats (all optional)", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Pair2(
                { Field(hp, { hp = it }, "Power (hp)", hpOk) },
                { Field(nm, { nm = it }, "Torque (Nm)", nmOk) },
            )
            Pair2(
                { Field(cc, { cc = it }, "Engine (cc)", ccOk) },
                { Field(kg, { kg = it }, "Weight (kg)", kgOk) },
            )
            Pair2(
                { Field(year, { year = it }, "Year", yearOk, integer = true) },
                { Field(tank, { tank = it }, "Tank (litres)", tankOk) },
            )
            Field(top, { top = it }, "Claimed top speed (km/h)", topOk, Modifier.fillMaxWidth())
            OutlinedTextField(notes, { notes = it }, label = { Text("Notes") }, modifier = Modifier.fillMaxWidth(), minLines = 2, maxLines = 4)
        }

        Button(
            onClick = {
                if (photoPath != originalPhoto) photos.delete(originalPhoto)
                onSave(
                    (initial ?: VehicleEntity(name = "")).copy(
                        name = name.trim(), kind = kind, hp = hpV, torqueNm = nmV, displacementCc = ccV, weightKg = kgV,
                        year = yearV, tankLiters = tankV, claimedTopKmh = topV, notes = notes.trim(), photoPath = photoPath,
                    ),
                )
            },
            enabled = valid,
            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp).navigationBarsPadding().height(56.dp),
            shape = RoundedCornerShape(28.dp),
            colors = ButtonDefaults.buttonColors(containerColor = Orange, contentColor = MaterialTheme.colorScheme.onPrimary),
        ) { Text("Save vehicle", style = MaterialTheme.typography.titleMedium) }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("Delete ${initial?.name}?") },
            text = { Text("Your rides and fill-ups are kept. They just won't be linked to a vehicle any more.") },
            confirmButton = {
                TextButton(onClick = { confirmDelete = false; discardPendingPhoto(); onDelete() }) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun Pair2(left: @Composable () -> Unit, right: @Composable () -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.weight(1f)) { left() }
        Column(Modifier.weight(1f)) { right() }
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, label: String, ok: Boolean, modifier: Modifier = Modifier.fillMaxWidth(), integer: Boolean = false) {
    OutlinedTextField(
        value, onChange, modifier = modifier, singleLine = true, isError = !ok, label = { Text(label, maxLines = 1) },
        keyboardOptions = KeyboardOptions(keyboardType = if (integer) KeyboardType.Number else KeyboardType.Decimal),
        supportingText = if (!ok) ({ Text("Enter a number") }) else null,
    )
}
