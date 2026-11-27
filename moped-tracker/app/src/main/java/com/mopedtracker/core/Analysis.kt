package com.mopedtracker.core

import com.mopedtracker.data.TrackPointEntity

data class HistogramBar(val fromUnit: Int, val toUnit: Int, val seconds: Double, val fraction: Float)

object Analysis {
    /**
     * How long was spent in each speed band. [unitPerMps] converts m/s into the display unit
     * (3.6 for km/h) and [step] is the band width in that unit.
     */
    fun speedHistogram(points: List<TrackPointEntity>, unitPerMps: Double, step: Int): List<HistogramBar> {
        if (points.size < 2) return emptyList()
        val seconds = HashMap<Int, Double>()
        for (i in 0 until points.size - 1) {
            val dt = ((points[i + 1].time - points[i].time) / 1000.0).coerceIn(0.0, 10.0)
            val band = (points[i].speedMps * unitPerMps / step).toInt()
            seconds[band] = (seconds[band] ?: 0.0) + dt
        }
        val total = seconds.values.sum()
        if (total <= 0.0) return emptyList()
        val top = seconds.keys.max()
        return (0..top).map { band ->
            val s = seconds[band] ?: 0.0
            HistogramBar(band * step, (band + 1) * step, s, (s / total).toFloat())
        }
    }

    /** (seconds since start, speed in display unit) pairs, thinned to at most [max] samples. */
    fun speedSeries(points: List<TrackPointEntity>, unitPerMps: Double, max: Int = 240): List<Pair<Float, Float>> {
        if (points.isEmpty()) return emptyList()
        val t0 = points.first().time
        return thin(points, max).map { ((it.time - t0) / 1000f) to (it.speedMps * unitPerMps).toFloat() }
    }

    /** Keeps at most [max] evenly spaced items so charts stay cheap to draw. */
    fun <T> thin(list: List<T>, max: Int): List<T> {
        if (list.size <= max || max < 2) return list
        val step = (list.size - 1).toDouble() / (max - 1)
        return List(max) { list[(it * step).toInt()] }
    }
}
