package com.mopedtracker.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.mopedtracker.MopedApp
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStatus
import com.mopedtracker.core.TrackerBus
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.service.TrackingService
import com.mopedtracker.ui.screens.DetailState
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

class MainViewModel(app: Application) : AndroidViewModel(app) {
    private val mopedApp = app as MopedApp
    private val repo = mopedApp.repository
    private val settings = mopedApp.settings

    val sessions: StateFlow<List<SessionEntity>> =
        repo.observeFinished().stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())
    val live: StateFlow<LiveState> = TrackerBus.state
    val unit: StateFlow<SpeedUnit> = settings.unitFlow
    val finished: SharedFlow<Long> = TrackerBus.finished

    init {
        // Close rides that were left open by a crash, unless the service is recording one right now.
        viewModelScope.launch {
            if (!TrackingService.running && TrackerBus.state.value.status == RideStatus.IDLE) repo.finalizeOrphans()
        }
    }

    fun toggleUnit() = settings.setUnit(if (settings.unit == SpeedUnit.KMH) SpeedUnit.MPH else SpeedUnit.KMH)

    fun startRide() = TrackingService.send(getApplication(), TrackingService.ACTION_START)
    fun stopRide() = TrackingService.send(getApplication(), TrackingService.ACTION_STOP)
    fun pauseResume() = TrackingService.send(
        getApplication(),
        if (live.value.status == RideStatus.PAUSED) TrackingService.ACTION_RESUME else TrackingService.ACTION_PAUSE,
    )

    fun detail(id: Long): Flow<DetailState> = repo.observeSession(id).map { s ->
        when {
            s == null -> DetailState.NotFound
            s.endTime == null -> DetailState.Loading // still being finalised by the service
            else -> DetailState.Ready(s, repo.getPoints(id))
        }
    }

    fun rename(id: Long, name: String) = viewModelScope.launch { repo.rename(id, name) }
    fun delete(id: Long) = viewModelScope.launch { repo.delete(id) }
}
