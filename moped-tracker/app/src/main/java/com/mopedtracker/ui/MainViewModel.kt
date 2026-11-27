package com.mopedtracker.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.mopedtracker.MopedApp
import com.mopedtracker.core.FuelCalc
import com.mopedtracker.core.FuelStats
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.Records
import com.mopedtracker.core.VehicleStats
import com.mopedtracker.core.VehicleStatsCalc
import com.mopedtracker.core.RideRecord
import com.mopedtracker.core.RideStatus
import com.mopedtracker.core.TrackerBus
import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.data.SessionEntity
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.data.VehicleEntity
import com.mopedtracker.data.VehiclePhotos
import com.mopedtracker.ui.screens.PhotoOps
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import com.mopedtracker.service.TrackingService
import com.mopedtracker.ui.screens.DetailState
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
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

    val fuel: StateFlow<List<FuelEntryEntity>> =
        repo.observeFuel().stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())
    private val _fuelVehicleFilter = MutableStateFlow<Long?>(null)
    val fuelVehicleFilter: StateFlow<Long?> = _fuelVehicleFilter.asStateFlow()
    fun setFuelVehicleFilter(id: Long?) { _fuelVehicleFilter.value = id }

    /** Fill-ups for the vehicle picked in the fuel screen's filter (all of them when none is picked). */
    val fuelShown: StateFlow<List<FuelEntryEntity>> = combine(fuel, _fuelVehicleFilter) { f, id -> if (id == null) f else f.filter { it.vehicleId == id } }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())
    val fuelStats: StateFlow<FuelStats> = combine(fuelShown, sessions) { f, s -> FuelCalc.compute(f, s) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), FuelStats())

    val vehicles: StateFlow<List<VehicleEntity>> =
        repo.observeVehicles().stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())
    val vehicleStats: StateFlow<Map<Long?, VehicleStats>> = sessions.map { VehicleStatsCalc.compute(it) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyMap())

    private val _homeVehicleFilter = MutableStateFlow<Long?>(null)
    val homeVehicleFilter: StateFlow<Long?> = _homeVehicleFilter.asStateFlow()
    fun setHomeVehicleFilter(id: Long?) { _homeVehicleFilter.value = id }

    val lastVehicleId: Long? get() = settings.lastVehicleId

    val photoOps = PhotoOps(
        import = { uri -> withContext(Dispatchers.IO) { VehiclePhotos.import(app, uri) } },
        delete = { path -> VehiclePhotos.delete(path) },
        cameraTarget = { VehiclePhotos.newCameraTarget(app) },
    )

    fun saveVehicle(vehicle: VehicleEntity) = viewModelScope.launch { repo.saveVehicle(vehicle) }

    fun deleteVehicle(id: Long) = viewModelScope.launch {
        val photo = repo.getVehicle(id)?.photoPath
        repo.deleteVehicle(id)
        VehiclePhotos.delete(photo)
        // a deleted vehicle can't remain an active filter
        if (_homeVehicleFilter.value == id) _homeVehicleFilter.value = null
        if (_fuelVehicleFilter.value == id) _fuelVehicleFilter.value = null
        if (settings.lastVehicleId == id) settings.lastVehicleId = null
    }
    val records: StateFlow<List<RideRecord>> = sessions.map { Records.compute(it) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    private val _starredOnly = MutableStateFlow(false)
    val starredOnly: StateFlow<Boolean> = _starredOnly.asStateFlow()
    fun toggleStarredFilter() { _starredOnly.value = !_starredOnly.value }

    fun toggleStar(id: Long, currentlyStarred: Boolean) = viewModelScope.launch { repo.setStarred(id, !currentlyStarred) }
    fun addFuel(entry: FuelEntryEntity) = viewModelScope.launch { repo.addFuel(entry) }
    fun deleteFuel(id: Long) = viewModelScope.launch { repo.deleteFuel(id) }
    suspend fun points(id: Long): List<TrackPointEntity> = repo.getPoints(id)

    fun toggleUnit() = settings.setUnit(if (settings.unit == SpeedUnit.KMH) SpeedUnit.MPH else SpeedUnit.KMH)

    /** Starts recording a ride on [vehicleId] (null = no vehicle) and remembers it as the default for next time. */
    fun startRide(vehicleId: Long?) {
        settings.lastVehicleId = vehicleId
        TrackingService.start(getApplication(), vehicleId)
    }
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
