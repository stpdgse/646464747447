package com.mopedtracker.data

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(tableName = "sessions")
data class SessionEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val name: String,
    val startTime: Long,
    /** null while the ride is still being recorded. */
    val endTime: Long? = null,
    val pausedMs: Long = 0,
    val distanceM: Double = 0.0,
    val elapsedMs: Long = 0,
    val movingMs: Long = 0,
    val maxSpeedMps: Float = 0f,
    val avgSpeedMps: Float = 0f,
    val avgMovingSpeedMps: Float = 0f,
    val elevationGainM: Double = 0.0,
    val elevationLossM: Double = 0.0,
    val maxAltitudeM: Double? = null,
    val minAltitudeM: Double? = null,
    val avgAccuracyM: Float = 0f,
    val pointCount: Int = 0,
    /** Down-sampled route as "lat,lon;lat,lon;..." for list thumbnails. */
    val previewRoute: String = "",
)

@Entity(
    tableName = "points",
    foreignKeys = [
        ForeignKey(
            entity = SessionEntity::class,
            parentColumns = ["id"],
            childColumns = ["sessionId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
    indices = [Index("sessionId")],
)
data class TrackPointEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val sessionId: Long,
    val time: Long,
    val lat: Double,
    val lon: Double,
    val altitude: Double?,
    val speedMps: Float,
    val accuracyM: Float,
    val speedAccuracyMps: Float?,
    val bearing: Float?,
)
