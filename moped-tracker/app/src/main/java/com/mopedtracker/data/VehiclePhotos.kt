package com.mopedtracker.data

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.net.Uri
import androidx.core.content.FileProvider
import androidx.exifinterface.media.ExifInterface
import java.io.File
import java.util.UUID
import kotlin.math.max

/** Stores vehicle photos privately in the app, downscaled so a garage full of vehicles stays small. */
object VehiclePhotos {
    private const val MAX_SIDE = 1024
    private const val JPEG_QUALITY = 88

    /** Copies [uri] (gallery pick or camera result) into app storage and returns the new file's path, or null if unreadable. */
    fun import(context: Context, uri: Uri): String? = runCatching {
        val resolver = context.contentResolver

        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, bounds) } ?: return null
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null

        val opts = BitmapFactory.Options().apply { inSampleSize = sampleSize(bounds.outWidth, bounds.outHeight, MAX_SIDE) }
        var bmp = resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, opts) } ?: return null

        val orientation = resolver.openInputStream(uri)?.use {
            ExifInterface(it).getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL)
        } ?: ExifInterface.ORIENTATION_NORMAL
        bmp = rotate(bmp, orientation)

        val longest = max(bmp.width, bmp.height)
        if (longest > MAX_SIDE) {
            val k = MAX_SIDE.toFloat() / longest
            bmp = Bitmap.createScaledBitmap(bmp, (bmp.width * k).toInt().coerceAtLeast(1), (bmp.height * k).toInt().coerceAtLeast(1), true)
        }

        val dir = File(context.filesDir, "vehicles").apply { mkdirs() }
        val out = File(dir, "${UUID.randomUUID()}.jpg")
        out.outputStream().use { bmp.compress(Bitmap.CompressFormat.JPEG, JPEG_QUALITY, it) }
        out.absolutePath
    }.getOrNull()

    fun delete(path: String?) {
        if (path != null) runCatching { File(path).delete() }
    }

    /** Decodes a stored photo, sampled down so it is no larger than roughly [maxSide] pixels. */
    fun decode(path: String, maxSide: Int): Bitmap? = runCatching {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        if (bounds.outWidth <= 0) return null
        BitmapFactory.decodeFile(path, BitmapFactory.Options().apply { inSampleSize = sampleSize(bounds.outWidth, bounds.outHeight, maxSide) })
    }.getOrNull()

    /** A fresh file + content Uri for the camera app to write a full-size picture into. */
    fun newCameraTarget(context: Context): Pair<File, Uri> {
        val dir = File(context.cacheDir, "camera").apply { mkdirs() }
        val file = File(dir, "capture_${UUID.randomUUID()}.jpg")
        return file to FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
    }

    private fun sampleSize(w: Int, h: Int, target: Int): Int {
        var s = 1
        while (max(w, h) / (s * 2) >= target) s *= 2
        return s
    }

    private fun rotate(bmp: Bitmap, orientation: Int): Bitmap {
        val m = Matrix()
        when (orientation) {
            ExifInterface.ORIENTATION_ROTATE_90 -> m.postRotate(90f)
            ExifInterface.ORIENTATION_ROTATE_180 -> m.postRotate(180f)
            ExifInterface.ORIENTATION_ROTATE_270 -> m.postRotate(270f)
            ExifInterface.ORIENTATION_FLIP_HORIZONTAL -> m.postScale(-1f, 1f)
            ExifInterface.ORIENTATION_FLIP_VERTICAL -> m.postScale(1f, -1f)
            ExifInterface.ORIENTATION_TRANSPOSE -> { m.postRotate(90f); m.postScale(-1f, 1f) }
            ExifInterface.ORIENTATION_TRANSVERSE -> { m.postRotate(270f); m.postScale(-1f, 1f) }
            else -> return bmp
        }
        return Bitmap.createBitmap(bmp, 0, 0, bmp.width, bmp.height, m, true)
    }
}
