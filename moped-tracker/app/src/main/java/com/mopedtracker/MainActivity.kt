package com.mopedtracker

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.location.LocationManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.NavType
import androidx.navigation.navArgument
import com.mopedtracker.core.RideStatus
import com.mopedtracker.ui.MainViewModel
import com.mopedtracker.data.TrackPointEntity
import com.mopedtracker.share.ShareCard
import com.mopedtracker.ui.screens.BigSpeedScreen
import com.mopedtracker.ui.screens.CompareScreen
import com.mopedtracker.ui.screens.DetailScreen
import com.mopedtracker.ui.screens.FuelScreen
import com.mopedtracker.ui.screens.RecordsScreen
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import com.mopedtracker.ui.screens.DetailState
import com.mopedtracker.ui.screens.HomeScreen
import com.mopedtracker.ui.screens.LiveScreen
import com.mopedtracker.ui.theme.MopedTheme

class MainActivity : ComponentActivity() {
    private val vm: MainViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { MopedTheme { AppNav(vm) } }
    }
}

private object Routes {
    const val HOME = "home"
    const val LIVE = "live"
    const val BIG = "big"
    const val FUEL = "fuel"
    const val RECORDS = "records"
    const val COMPARE = "compare"
    const val DETAIL = "detail/{id}"
    fun detail(id: Long) = "detail/$id"
}

@Composable
private fun AppNav(vm: MainViewModel) {
    val nav = rememberNavController()
    val sessions by vm.sessions.collectAsStateWithLifecycle()
    val live by vm.live.collectAsStateWithLifecycle()
    val unit by vm.unit.collectAsStateWithLifecycle()
    val starredOnly by vm.starredOnly.collectAsStateWithLifecycle()
    val fuel by vm.fuel.collectAsStateWithLifecycle()
    val fuelStats by vm.fuelStats.collectAsStateWithLifecycle()
    val records by vm.records.collectAsStateWithLifecycle()
    val starter = rememberRideStarter(onGranted = {
        vm.startRide()
        nav.navigate(Routes.LIVE)
    })

    fun onRideScreen() = nav.currentDestination?.route.let { it == Routes.LIVE || it == Routes.BIG }

    // A ride was saved: jump from the live (or big speed) screen to its summary.
    LaunchedEffect(Unit) {
        vm.finished.collect { id ->
            if (onRideScreen()) nav.navigate(Routes.detail(id)) { popUpTo(Routes.HOME) }
        }
    }
    // If the ride ends while a ride screen is open (e.g. Stop from the notification) and nothing navigated, go home.
    LaunchedEffect(live.status) {
        if (live.status == RideStatus.IDLE && onRideScreen()) nav.popBackStack(Routes.HOME, false)
    }

    NavHost(nav, startDestination = Routes.HOME) {
        composable(Routes.HOME) {
            HomeScreen(
                sessions = sessions,
                live = live,
                unit = unit,
                starredOnly = starredOnly,
                onStart = starter,
                onOpenLive = { nav.navigate(Routes.LIVE) },
                onOpenSession = { nav.navigate(Routes.detail(it)) },
                onToggleUnit = vm::toggleUnit,
                onToggleStarred = vm::toggleStarredFilter,
                onOpenRecords = { nav.navigate(Routes.RECORDS) },
                onOpenFuel = { nav.navigate(Routes.FUEL) },
                onOpenCompare = { nav.navigate(Routes.COMPARE) },
            )
        }
        composable(Routes.LIVE) {
            KeepScreenOn()
            LiveScreen(
                live = live,
                unit = unit,
                onPauseResume = vm::pauseResume,
                onStop = vm::stopRide,
                onBack = { nav.popBackStack() },
                onBigMode = { nav.navigate(Routes.BIG) },
            )
        }
        composable(Routes.BIG) {
            KeepScreenOn()
            BigSpeedScreen(live = live, unit = unit, onClose = { nav.popBackStack() })
        }
        composable(Routes.FUEL) {
            FuelScreen(
                entries = fuel,
                stats = fuelStats,
                unit = unit,
                onBack = { nav.popBackStack() },
                onAdd = vm::addFuel,
                onDelete = vm::deleteFuel,
            )
        }
        composable(Routes.RECORDS) {
            RecordsScreen(
                records = records,
                starred = sessions.filter { it.starred },
                unit = unit,
                onBack = { nav.popBackStack() },
                onOpen = { nav.navigate(Routes.detail(it)) },
            )
        }
        composable(Routes.COMPARE) {
            // Default to the two most recent rides; the user can change either side.
            var idA by rememberSaveable { mutableStateOf<Long?>(null) }
            var idB by rememberSaveable { mutableStateOf<Long?>(null) }
            val a = sessions.firstOrNull { it.id == (idA ?: sessions.getOrNull(0)?.id) }
            val b = sessions.firstOrNull { it.id == (idB ?: sessions.getOrNull(1)?.id) }
            val pointsA by produceState(emptyList<TrackPointEntity>(), a?.id) { value = a?.let { vm.points(it.id) } ?: emptyList() }
            val pointsB by produceState(emptyList<TrackPointEntity>(), b?.id) { value = b?.let { vm.points(it.id) } ?: emptyList() }
            CompareScreen(
                sessions = sessions,
                a = a, b = b, pointsA = pointsA, pointsB = pointsB, unit = unit,
                onPickA = { idA = it },
                onPickB = { idB = it },
                onBack = { nav.popBackStack() },
            )
        }
        composable(Routes.DETAIL, arguments = listOf(navArgument("id") { type = NavType.LongType })) { entry ->
            val id = entry.arguments!!.getLong("id")
            val state by remember(id) { vm.detail(id) }.collectAsState(initial = DetailState.Loading)
            val context = LocalContext.current
            val scope = rememberCoroutineScope()
            DetailScreen(
                state = state,
                unit = unit,
                onBack = { nav.popBackStack() },
                onRename = { vm.rename(id, it) },
                onDelete = { vm.delete(id); nav.popBackStack() },
                onToggleStar = { (state as? DetailState.Ready)?.let { vm.toggleStar(id, it.session.starred) } },
                onShare = { session, points ->
                    scope.launch {
                        val card = withContext(Dispatchers.Default) { ShareCard.render(session, points, unit) }
                        ShareCard.share(context, card, session.id, "${session.name} - tracked with Moped Tracker")
                    }
                },
            )
        }
    }
}

@Composable
private fun KeepScreenOn() {
    val view = LocalView.current
    DisposableEffect(view) {
        view.keepScreenOn = true
        onDispose { view.keepScreenOn = false }
    }
}

/**
 * Returns a lambda that starts a ride after making sure we have precise location, notification permission
 * (Android 13+) and that the device's GPS is switched on.
 */
@Composable
private fun rememberRideStarter(onGranted: () -> Unit): () -> Unit {
    val context = LocalContext.current
    var dialog by remember { mutableStateOf<String?>(null) }

    fun gpsEnabled() = (context.getSystemService(LocationManager::class.java)).isProviderEnabled(LocationManager.GPS_PROVIDER)
    fun hasFine() = ContextCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED

    fun proceed() {
        if (!gpsEnabled()) dialog = "gps" else onGranted()
    }

    val launcher = rememberRideLauncher { granted -> if (granted) proceed() else dialog = "permission" }

    dialog?.let { kind ->
        val gps = kind == "gps"
        AlertDialog(
            onDismissRequest = { dialog = null },
            title = { Text(if (gps) "Turn on location" else "Precise location needed") },
            text = {
                Text(
                    if (gps) "GPS is switched off. Turn on location services so your speed can be measured."
                    else "Allow precise location so the app can measure your real speed and route. You can change this in system settings.",
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    dialog = null
                    val intent = if (gps) Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)
                    else Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
                    context.startActivity(intent)
                }) { Text("Open settings") }
            },
            dismissButton = { TextButton(onClick = { dialog = null }) { Text("Not now") } },
        )
    }

    return {
        if (hasFine()) proceed()
        else {
            val perms = buildList {
                add(Manifest.permission.ACCESS_FINE_LOCATION)
                add(Manifest.permission.ACCESS_COARSE_LOCATION)
                if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
            }
            launcher(perms.toTypedArray())
        }
    }
}

@Composable
private fun rememberRideLauncher(onResult: (Boolean) -> Unit): (Array<String>) -> Unit {
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { result ->
        onResult(result[Manifest.permission.ACCESS_FINE_LOCATION] == true)
    }
    return { launcher.launch(it) }
}
