package com.mopedtracker.core

import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min

/**
 * Fits lat/lon points into a [width] x [height] box with [pad] margin, keeping the real-world aspect ratio.
 * Used by both the Compose route views and the share-image renderer.
 */
class RouteProjection(points: List<LatLon>, private val width: Float, private val height: Float, pad: Float) {
    private val minLat = points.minOf { it.lat }
    private val maxLat = points.maxOf { it.lat }
    private val minLon = points.minOf { it.lon }
    private val maxLon = points.maxOf { it.lon }
    private val lonScale = cos(Math.toRadians((minLat + maxLat) / 2))
    private val spanX = max((maxLon - minLon) * lonScale, 1e-6)
    private val spanY = max(maxLat - minLat, 1e-6)
    private val scale = min((width - 2 * pad) / spanX, (height - 2 * pad) / spanY)
    private val offX = (width - spanX * scale) / 2
    private val offY = (height - spanY * scale) / 2

    fun x(p: LatLon): Float = (offX + (p.lon - minLon) * lonScale * scale).toFloat()
    fun y(p: LatLon): Float = (height - (offY + (p.lat - minLat) * scale)).toFloat()
}
