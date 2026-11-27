package com.mopedtracker.ui

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.text.DateFormat
import java.util.Date
import java.util.Locale

enum class SpeedUnit(val label: String, val distanceLabel: String, val perMps: Double, val perMetre: Double, val histogramStep: Int) {
    KMH("km/h", "km", 3.6, 0.001, 10),
    MPH("mph", "mi", 2.2369363, 0.000621371, 5),
}

/** Tiny persisted settings store (only the unit system for now). */
class Settings(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
    private val _unit = MutableStateFlow(
        runCatching { SpeedUnit.valueOf(prefs.getString("unit", SpeedUnit.KMH.name)!!) }.getOrDefault(SpeedUnit.KMH),
    )
    val unitFlow: StateFlow<SpeedUnit> = _unit.asStateFlow()
    val unit: SpeedUnit get() = _unit.value

    /** The vehicle chosen for the previous ride, preselected in the start dialog. */
    var lastVehicleId: Long?
        get() = prefs.getLong("lastVehicleId", -1L).takeIf { it >= 0 }
        set(v) = prefs.edit { if (v == null) remove("lastVehicleId") else putLong("lastVehicleId", v) }

    fun setUnit(unit: SpeedUnit) {
        prefs.edit { putString("unit", unit.name) }
        _unit.value = unit
    }
}

object Format {
    private val loc = Locale.US

    /** Speed as a bare number in the chosen unit. */
    fun speed(mps: Float, unit: SpeedUnit, decimals: Int = 0): String =
        String.format(loc, "%.${decimals}f", mps * unit.perMps)

    fun speedValue(mps: Float, unit: SpeedUnit): Float = (mps * unit.perMps).toFloat()

    fun distance(metres: Double, unit: SpeedUnit): String {
        val v = metres * unit.perMetre
        return String.format(loc, if (v < 10) "%.2f" else "%.1f", v)
    }

    fun duration(ms: Long): String {
        val total = ms / 1000
        val h = total / 3600
        val m = (total % 3600) / 60
        val s = total % 60
        return if (h > 0) String.format(loc, "%d:%02d:%02d", h, m, s) else String.format(loc, "%02d:%02d", m, s)
    }

    fun metres(v: Double): String = String.format(loc, "%.0f", v)

    /** Money in the device's currency, e.g. "€4.80". */
    fun money(v: Double): String = java.text.NumberFormat.getCurrencyInstance().format(v)

    fun liters(v: Double): String = String.format(loc, "%.2f", v)

    /** 12.0 -> "12", 12.5 -> "12.5". */
    fun number(v: Double): String = if (v % 1.0 == 0.0) v.toLong().toString() else String.format(loc, "%.1f", v)

    /** Offset into a ride as m:ss or h:mm:ss. */
    fun offset(ms: Long): String = duration(ms)

    fun dateTime(ms: Long): String =
        DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT).format(Date(ms))

    fun date(ms: Long): String = DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date(ms))

    fun time(ms: Long): String = DateFormat.getTimeInstance(DateFormat.SHORT).format(Date(ms))
}
