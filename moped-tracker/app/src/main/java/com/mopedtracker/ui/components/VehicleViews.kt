package com.mopedtracker.ui.components

import android.util.LruCache
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.TwoWheeler
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.data.VehiclePhotos
import com.mopedtracker.ui.Format
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Small in-memory cache so scrolling a list doesn't re-decode the same JPEGs. */
object VehiclePhotoCache {
    private val lru = LruCache<String, ImageBitmap>(24)
    private fun key(path: String, maxSide: Int) = "$path@$maxSide"

    fun get(path: String, maxSide: Int): ImageBitmap? = lru.get(key(path, maxSide))

    fun getOrDecode(path: String, maxSide: Int): ImageBitmap? =
        get(path, maxSide) ?: VehiclePhotos.decode(path, maxSide)?.asImageBitmap()?.also { lru.put(key(path, maxSide), it) }
}

/** A vehicle photo cropped to fill its box, or a neutral placeholder when there is none. */
@Composable
fun VehiclePhoto(path: String?, modifier: Modifier = Modifier, maxSide: Int = 640) {
    val cached = remember(path, maxSide) { path?.let { VehiclePhotoCache.get(it, maxSide) } }
    val bitmap by produceState(cached, path, maxSide) {
        if (value == null && path != null) value = withContext(Dispatchers.IO) { VehiclePhotoCache.getOrDecode(path, maxSide) }
    }
    Box(modifier.background(MaterialTheme.colorScheme.surfaceVariant)) {
        val b = bitmap
        if (b != null) {
            Image(b, contentDescription = null, modifier = Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        } else {
            Icon(
                Icons.Filled.TwoWheeler, null,
                Modifier.align(Alignment.Center).size(36.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            )
        }
    }
}

/** e.g. ["12 hp", "16 Nm", "125 cc"], only for the stats that were filled in. */
fun VehicleEntity.specParts(): List<String> = listOfNotNull(
    hp?.let { "${Format.number(it)} hp" },
    torqueNm?.let { "${Format.number(it)} Nm" },
    displacementCc?.let { "${Format.number(it)} cc" },
    weightKg?.let { "${Format.number(it)} kg" },
    tankLiters?.let { "${Format.number(it)} L tank" },
    claimedTopKmh?.let { "${Format.number(it)} km/h top" },
)

fun VehicleEntity.subtitle(): String = listOfNotNull(kind, year?.toString()).joinToString(" · ")
