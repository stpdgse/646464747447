package com.mopedtracker.service

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import android.os.SystemClock
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationResult
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.mopedtracker.MainActivity
import com.mopedtracker.MopedApp
import com.mopedtracker.R
import com.mopedtracker.core.Fix
import com.mopedtracker.core.GpsQuality
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStatus
import com.mopedtracker.core.StatsAccumulator
import com.mopedtracker.core.TrackerBus
import com.mopedtracker.data.RideRepository
import com.mopedtracker.data.RouteCodec
import com.mopedtracker.data.toFix
import com.mopedtracker.ui.Format
import com.mopedtracker.ui.SpeedUnit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Records a ride in the foreground so it keeps running with the screen off or the app in the background.
 * Every accepted GPS point is written to the database immediately, so a killed process loses nothing.
 */
class TrackingService : Service() {

    companion object {
        const val ACTION_START = "com.mopedtracker.action.START"
        const val ACTION_PAUSE = "com.mopedtracker.action.PAUSE"
        const val ACTION_RESUME = "com.mopedtracker.action.RESUME"
        const val ACTION_STOP = "com.mopedtracker.action.STOP"

        private const val CHANNEL_ID = "tracking"
        private const val NOTIFICATION_ID = 1
        private const val MAX_LIVE_ROUTE_POINTS = 400

        @Volatile var running = false
            private set

        fun send(context: Context, action: String) {
            val intent = Intent(context, TrackingService::class.java).setAction(action)
            ContextCompat.startForegroundService(context, intent)
        }
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private val fused by lazy { LocationServices.getFusedLocationProviderClient(this) }
    private val repo: RideRepository get() = (application as MopedApp).repository

    private var accumulator = StatsAccumulator()
    private var sessionId = 0L
    private var startMs = 0L
    private var pausedMs = 0L
    private var pauseStartedAt = 0L
    private var status = RideStatus.IDLE
    private var currentSpeed = 0f
    private var lastAccuracy: Float? = null
    private val route = ArrayList<LatLon>()
    private var ticker: Job? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var lastNotifiedAt = 0L
    private var lastFixAt = SystemClock.elapsedRealtime()

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        running = true
        val channel = NotificationChannel(CHANNEL_ID, "Ride recording", NotificationManager.IMPORTANCE_LOW).apply {
            description = "Shown while a ride is being recorded"
            setShowBadge(false)
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Must be promoted to the foreground immediately after startForegroundService().
        enterForeground()
        when (intent?.action) {
            ACTION_START -> if (status == RideStatus.IDLE) startRide()
            ACTION_PAUSE -> pauseRide()
            ACTION_RESUME -> resumeRide()
            ACTION_STOP -> stopRide()
            null -> recoverRide() // restarted by the system after the process was killed
        }
        return START_STICKY
    }

    override fun onDestroy() {
        running = false
        stopLocationUpdates()
        ticker?.cancel()
        releaseWakeLock()
        scope.cancel()
        super.onDestroy()
    }

    // region ride lifecycle

    private fun startRide() {
        status = RideStatus.RECORDING // claim immediately so a double tap can't start two rides
        scope.launch {
            repo.finalizeOrphans()
            val now = System.currentTimeMillis()
            sessionId = repo.createSession(now)
            startMs = now
            pausedMs = 0
            accumulator = StatsAccumulator()
            route.clear()
            beginRecording()
        }
    }

    private fun recoverRide() {
        scope.launch {
            val active = repo.getActiveSessionOrNull()
            if (active == null) {
                stopSelf()
                return@launch
            }
            sessionId = active.id
            startMs = active.startTime
            pausedMs = active.pausedMs
            accumulator = StatsAccumulator()
            route.clear()
            repo.getPoints(active.id).forEach {
                accumulator.add(it.toFix())
                route.add(LatLon(it.lat, it.lon))
            }
            beginRecording()
        }
    }

    private fun beginRecording() {
        status = RideStatus.RECORDING
        acquireWakeLock()
        publish()
        if (!startLocationUpdates()) {
            // Permission revoked between tap and start.
            scope.launch { repo.finish(sessionId, System.currentTimeMillis(), pausedMs, accumulator.snapshot(0)) }
            resetAndStop()
            return
        }
        startTicker()
    }

    private fun pauseRide() {
        if (status != RideStatus.RECORDING) return
        status = RideStatus.PAUSED
        pauseStartedAt = System.currentTimeMillis()
        stopLocationUpdates()
        currentSpeed = 0f
        publish()
        updateNotification(force = true)
    }

    private fun resumeRide() {
        if (status != RideStatus.PAUSED) return
        pausedMs += System.currentTimeMillis() - pauseStartedAt
        accumulator.breakLink()
        status = RideStatus.RECORDING
        scope.launch { repo.savePausedMs(sessionId, pausedMs) }
        startLocationUpdates()
        publish()
        updateNotification(force = true)
    }

    private fun stopRide() {
        if (status == RideStatus.IDLE) {
            resetAndStop()
            return
        }
        if (status == RideStatus.PAUSED) pausedMs += System.currentTimeMillis() - pauseStartedAt
        stopLocationUpdates()
        ticker?.cancel()
        val id = sessionId
        val end = System.currentTimeMillis()
        val stats = accumulator.snapshot(elapsedMs(end))
        val paused = pausedMs
        scope.launch {
            repo.finish(id, end, paused, stats)
            TrackerBus.emitFinished(id)
            resetAndStop()
        }
    }

    private fun resetAndStop() {
        status = RideStatus.IDLE
        TrackerBus.update { LiveState() }
        releaseWakeLock()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    // endregion

    // region location

    @SuppressLint("MissingPermission")
    private fun startLocationUpdates(): Boolean = try {
        val request = LocationRequest.Builder(Priority.PRIORITY_HIGH_ACCURACY, 1000L)
            .setMinUpdateIntervalMillis(500L)
            .setMinUpdateDistanceMeters(0f)
            .setWaitForAccurateLocation(false)
            .build()
        fused.requestLocationUpdates(request, callback, Looper.getMainLooper())
        true
    } catch (e: SecurityException) {
        false
    }

    private fun stopLocationUpdates() {
        fused.removeLocationUpdates(callback)
    }

    private val callback = object : LocationCallback() {
        override fun onLocationResult(result: LocationResult) {
            if (status != RideStatus.RECORDING) return
            for (loc in result.locations) onLocation(loc)
            publish()
            updateNotification()
        }
    }

    private fun onLocation(loc: android.location.Location) {
        val fix = Fix(
            timeMs = loc.time,
            lat = loc.latitude,
            lon = loc.longitude,
            altitudeM = if (loc.hasAltitude()) loc.altitude else null,
            speedMps = if (loc.hasSpeed()) loc.speed else null,
            accuracyM = if (loc.hasAccuracy()) loc.accuracy else 50f,
            speedAccuracyMps = if (Build.VERSION.SDK_INT >= 26 && loc.hasSpeedAccuracy()) loc.speedAccuracyMetersPerSecond else null,
            bearingDeg = if (loc.hasBearing()) loc.bearing else null,
        )
        lastAccuracy = fix.accuracyM
        lastFixAt = SystemClock.elapsedRealtime()
        val result = accumulator.add(fix)
        if (!result.accepted) return
        currentSpeed = result.speedMps
        route.add(LatLon(fix.lat, fix.lon))
        val id = sessionId
        scope.launch(Dispatchers.IO) { repo.addPoint(id, fix, result.speedMps) }
    }

    // endregion

    // region state, ticker, notification

    private fun elapsedMs(now: Long = System.currentTimeMillis()): Long {
        val pausedNow = if (status == RideStatus.PAUSED) now - pauseStartedAt else 0L
        return (now - startMs - pausedMs - pausedNow).coerceAtLeast(0)
    }

    private fun publish() {
        val acc = lastAccuracy
        val gps = when {
            acc == null -> GpsQuality.SEARCHING
            acc <= 10f -> GpsQuality.GOOD
            acc <= 25f -> GpsQuality.OK
            else -> GpsQuality.WEAK
        }
        TrackerBus.update {
            LiveState(
                status = status,
                sessionId = sessionId,
                startTimeMs = startMs,
                currentSpeedMps = if (status == RideStatus.RECORDING) currentSpeed else 0f,
                stats = accumulator.snapshot(elapsedMs()),
                accuracyM = acc,
                gps = gps,
                route = RouteCodec.downsample(route.toList(), MAX_LIVE_ROUTE_POINTS),
            )
        }
    }

    private fun startTicker() {
        ticker?.cancel()
        ticker = scope.launch {
            while (true) {
                delay(1000)
                // Without fresh fixes (tunnel, GPS lost) the speed must decay to 0 instead of sticking.
                if (status == RideStatus.RECORDING && SystemClock.elapsedRealtime() - lastFixAt > 4000) currentSpeed = 0f
                publish()
                updateNotification()
            }
        }
    }

    private fun enterForeground() {
        val n = buildNotification()
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTIFICATION_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
        } else {
            startForeground(NOTIFICATION_ID, n)
        }
    }

    private fun updateNotification(force: Boolean = false) {
        val now = SystemClock.elapsedRealtime()
        if (!force && now - lastNotifiedAt < 2000) return
        lastNotifiedAt = now
        getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, buildNotification())
    }

    private fun buildNotification(): Notification {
        val units = (application as MopedApp).settings.unit
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val b = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentIntent(open)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)

        when (status) {
            RideStatus.IDLE -> b.setContentTitle("Starting ride…")
            RideStatus.RECORDING -> {
                val st = accumulator.snapshot(elapsedMs())
                b.setContentTitle("Recording ride • ${Format.speed(currentSpeed, units)} ${units.label}")
                    .setContentText("${Format.distance(st.distanceM, units)} • ${Format.duration(st.elapsedMs)}")
                    .addAction(0, "Pause", actionIntent(ACTION_PAUSE))
                    .addAction(0, "Stop", actionIntent(ACTION_STOP))
            }
            RideStatus.PAUSED -> {
                b.setContentTitle("Ride paused")
                    .addAction(0, "Resume", actionIntent(ACTION_RESUME))
                    .addAction(0, "Stop", actionIntent(ACTION_STOP))
            }
        }
        return b.build()
    }

    private fun actionIntent(action: String): PendingIntent = PendingIntent.getService(
        this, action.hashCode(), Intent(this, TrackingService::class.java).setAction(action),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    private fun acquireWakeLock() {
        if (wakeLock?.isHeld == true) return
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "MopedTracker:ride").apply {
            setReferenceCounted(false)
            acquire(12 * 60 * 60 * 1000L)
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.takeIf { it.isHeld }?.release()
        wakeLock = null
    }

    // endregion
}
