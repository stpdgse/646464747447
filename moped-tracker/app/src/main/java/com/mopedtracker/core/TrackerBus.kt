package com.mopedtracker.core

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow

enum class RideStatus { IDLE, RECORDING, PAUSED }

enum class GpsQuality { SEARCHING, WEAK, OK, GOOD }

data class LiveState(
    val status: RideStatus = RideStatus.IDLE,
    val sessionId: Long? = null,
    val startTimeMs: Long = 0,
    val currentSpeedMps: Float = 0f,
    val stats: RideStats = RideStats(),
    val accuracyM: Float? = null,
    val gps: GpsQuality = GpsQuality.SEARCHING,
    /** Down-sampled route so far, for the live mini map. */
    val route: List<LatLon> = emptyList(),
)

/** Process-wide channel between the tracking service and the UI. */
object TrackerBus {
    private val _state = MutableStateFlow(LiveState())
    val state: StateFlow<LiveState> = _state.asStateFlow()

    private val _finished = MutableSharedFlow<Long>(extraBufferCapacity = 4)
    /** Emits the id of a ride once it has been saved. */
    val finished: SharedFlow<Long> = _finished.asSharedFlow()

    fun update(transform: (LiveState) -> LiveState) {
        _state.value = transform(_state.value)
    }

    fun emitFinished(id: Long) { _finished.tryEmit(id) }
}
