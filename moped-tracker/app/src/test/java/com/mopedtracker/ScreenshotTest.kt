package com.mopedtracker

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.github.takahirom.roborazzi.captureRoboImage
import com.mopedtracker.core.FuelCalc
import com.mopedtracker.core.GpsQuality
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.Records
import com.mopedtracker.core.RideStats
import com.mopedtracker.core.RideStatus
import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.share.ShareCard
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.screens.BigSpeedScreen
import com.mopedtracker.ui.screens.CompareScreen
import com.mopedtracker.ui.screens.DetailScreen
import com.mopedtracker.ui.screens.DetailState
import com.mopedtracker.ui.screens.FuelScreen
import com.mopedtracker.ui.screens.HomeScreen
import com.mopedtracker.ui.screens.LiveScreen
import com.mopedtracker.ui.screens.RecordsScreen
import com.mopedtracker.ui.theme.MopedTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import java.io.File
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.unit.dp

@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [35], qualifiers = "w411dp-h891dp-xxhdpi")
class ScreenshotTest {
    @get:Rule val rule = createComposeRule()

    private val rideData = listOf(
        SampleData.session(3, "Morning commute", 0.0, starred = true),
        SampleData.session(2, "Sunday cruise", 1.3),
        SampleData.session(1, "Ride Nov 20, 2025", 2.1, starred = true),
    )
    private val rides = rideData.map { it.first }
    private val idle = LiveState()

    private fun shot(name: String) = rule.onRoot().captureRoboImage("build/screenshots/$name.png")

    // region home

    @Test fun homeDark() {
        rule.setContent { MopedTheme(true) { HomeScreen(rides, idle, SpeedUnit.KMH, false, {}, {}, {}, {}) } }
        shot("home_dark")
    }

    @Test fun homeStarredFilter() {
        rule.setContent { MopedTheme(true) { HomeScreen(rides, idle, SpeedUnit.KMH, starredOnly = true, onStart = {}, onOpenLive = {}, onOpenSession = {}, onToggleUnit = {}) } }
        shot("home_starred")
    }

    @Test fun homeEmpty() {
        rule.setContent { MopedTheme(true) { HomeScreen(emptyList(), idle, SpeedUnit.KMH, false, {}, {}, {}, {}) } }
        shot("home_empty")
    }

    @Test fun homeLight() {
        rule.setContent { MopedTheme(false) { HomeScreen(rides, idle, SpeedUnit.MPH, false, {}, {}, {}, {}) } }
        shot("home_light_mph")
    }

    // endregion

    // region live + big mode

    private fun liveState(status: RideStatus, speed: Float = 12.4f) = LiveState(
        status = status, sessionId = 9, currentSpeedMps = speed,
        stats = RideStats(
            distanceM = 4_820.0, elapsedMs = 612_000, movingMs = 560_000, maxSpeedMps = 15.8f,
            avgSpeedMps = 7.9f, avgMovingSpeedMps = 8.6f, elevationGainM = 24.0,
        ),
        accuracyM = 4f, gps = GpsQuality.GOOD,
        route = SampleData.points().filterIndexed { i, _ -> i % 8 == 0 }.map { LatLon(it.lat, it.lon) },
    )

    @Test fun liveRecording() {
        rule.setContent { MopedTheme(true) { LiveScreen(liveState(RideStatus.RECORDING), SpeedUnit.KMH, {}, {}, {}) } }
        shot("live_recording")
    }

    @Test fun livePaused() {
        rule.setContent { MopedTheme(true) { LiveScreen(liveState(RideStatus.PAUSED), SpeedUnit.KMH, {}, {}, {}) } }
        shot("live_paused")
    }

    @Test fun bigSpeedPortrait() {
        rule.setContent { MopedTheme(true) { BigSpeedScreen(liveState(RideStatus.RECORDING, 12.4f), SpeedUnit.KMH) {} } }
        shot("big_speed")
    }

    @Test @Config(sdk = [35], qualifiers = "w891dp-h411dp-xxhdpi")
    fun bigSpeedLandscapeThreeDigits() {
        rule.setContent { MopedTheme(true) { BigSpeedScreen(liveState(RideStatus.RECORDING, 30.5f), SpeedUnit.KMH) {} } }
        shot("big_speed_landscape")
    }

    // endregion

    // region detail

    @Test fun detailDark() {
        val (s, p) = rideData[0]
        rule.setContent { MopedTheme(true) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.KMH, {}, {}, {}) } }
        shot("detail_dark")
    }

    /** A very tall screen so the whole page, including the Events card, is captured. */
    @Test @Config(sdk = [35], qualifiers = "w411dp-h2400dp-xxhdpi")
    fun detailFullPage() {
        val (s, p) = rideData[0]
        rule.setContent { MopedTheme(true) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.KMH, {}, {}, {}) } }
        shot("detail_full")
    }

    @Test fun detailLightMph() {
        val (s, p) = rideData[0]
        rule.setContent { MopedTheme(false) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.MPH, {}, {}, {}) } }
        shot("detail_light_mph")
    }

    // endregion

    // region fuel

    private val fuel = listOf(
        FuelEntryEntity(1, SampleData.points().first().time - 12 * 86_400_000L, 3.6, 5.94, true, "Shell"),
        FuelEntryEntity(2, SampleData.points().first().time - 3 * 86_400_000L, 1.1, 1.87, false),
        FuelEntryEntity(3, SampleData.points().last().time + 86_400_000L, 0.8, 1.38, true, "Aral"),
    )

    @Test fun fuelWithData() {
        val stats = FuelCalc.compute(fuel, rides)
        rule.setContent { MopedTheme(true) { FuelScreen(fuel, stats, SpeedUnit.KMH, {}, {}, {}) } }
        shot("fuel")
    }

    @Test fun fuelEmpty() {
        rule.setContent { MopedTheme(true) { FuelScreen(emptyList(), FuelCalc.compute(emptyList(), rides), SpeedUnit.KMH, {}, {}, {}) } }
        shot("fuel_empty")
    }

    // endregion

    // region records + compare

    @Test fun records() {
        rule.setContent { MopedTheme(true) { RecordsScreen(Records.compute(rides), rides.filter { it.starred }, SpeedUnit.KMH, {}, {}) } }
        shot("records")
    }

    @Test fun compare() {
        val (a, pa) = rideData[0]
        val (b, pb) = SampleData.session(2, "Sunday cruise", 1.3).let { (s, p) ->
            // make B a calmer, slower ride so the comparison has something to show
            s.copy(maxSpeedMps = s.maxSpeedMps * 0.8f, avgMovingSpeedMps = s.avgMovingSpeedMps * 0.85f, distanceM = s.distanceM * 0.9) to
                p.map { it.copy(speedMps = it.speedMps * 0.8f) }
        }
        rule.setContent { MopedTheme(true) { CompareScreen(rides, a, b, pa, pb, SpeedUnit.KMH, {}, {}, {}) } }
        shot("compare")
    }

    // endregion

    // region share card (plain Android graphics, no Compose)

    @Test fun shareCardRendersAndSaves() {
        val (s, p) = rideData[0]
        val bmp = ShareCard.render(s, p, SpeedUnit.KMH)
        assertEquals(ShareCard.WIDTH, bmp.width)
        assertEquals(ShareCard.HEIGHT, bmp.height)
        val out = File("build/screenshots/share_card.png").apply { parentFile?.mkdirs() }
        out.outputStream().use { bmp.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
        assertTrue(out.length() > 20_000)
    }

    // endregion

    // region vehicles

    private val context: android.content.Context get() = androidx.test.core.app.ApplicationProvider.getApplicationContext()
    private val garage by lazy {
        SampleData.vehicles(context).also { vs ->
            // pre-decode the photos at the sizes the screens use, so screenshots don't race the background loader
            vs.mapNotNull { it.photoPath }.forEach { path -> listOf(200, 240, 640, 900, 1024).forEach { com.mopedtracker.ui.components.VehiclePhotoCache.getOrDecode(path, it) } }
        }
    }
    private val vehicleRides by lazy {
        listOf(
            rides[0].copy(vehicleId = 1),
            rides[1].copy(vehicleId = 2, maxSpeedMps = 12.2f),
            rides[2].copy(vehicleId = 1),
        )
    }
    private val noPhotoOps = com.mopedtracker.ui.screens.PhotoOps(
        import = { null }, delete = {}, cameraTarget = { File("none.jpg") to android.net.Uri.EMPTY },
    )

    @Test fun garage() {
        val stats = com.mopedtracker.core.VehicleStatsCalc.compute(vehicleRides)
        rule.setContent { MopedTheme(true) { com.mopedtracker.ui.screens.GarageScreen(garage, stats, SpeedUnit.KMH, {}, {}, {}) } }
        shot("garage")
    }

    @Test @Config(sdk = [35], qualifiers = "w411dp-h1700dp-xxhdpi")
    fun garageFullPage() {
        val stats = com.mopedtracker.core.VehicleStatsCalc.compute(vehicleRides)
        rule.setContent { MopedTheme(true) { com.mopedtracker.ui.screens.GarageScreen(garage, stats, SpeedUnit.KMH, {}, {}, {}) } }
        shot("garage_full")
    }

    @Test fun garageEmpty() {
        rule.setContent { MopedTheme(true) { com.mopedtracker.ui.screens.GarageScreen(emptyList(), emptyMap(), SpeedUnit.KMH, {}, {}, {}) } }
        shot("garage_empty")
    }

    @Test fun vehicleEditExisting() {
        rule.setContent { MopedTheme(true) { com.mopedtracker.ui.screens.VehicleEditScreen(garage[0], noPhotoOps, {}, {}, {}) } }
        shot("vehicle_edit")
    }

    @Test @Config(sdk = [35], qualifiers = "w411dp-h1400dp-xxhdpi")
    fun vehicleEditNewFullPage() {
        rule.setContent { MopedTheme(true) { com.mopedtracker.ui.screens.VehicleEditScreen(null, noPhotoOps, {}, {}, {}) } }
        shot("vehicle_new_full")
    }

    @Test fun vehiclePicker() {
        rule.setContent {
            MopedTheme(true) {
                androidx.compose.foundation.layout.Box(androidx.compose.ui.Modifier.fillMaxSize().padding(20.dp), contentAlignment = androidx.compose.ui.Alignment.Center) {
                    androidx.compose.material3.Card(shape = androidx.compose.foundation.shape.RoundedCornerShape(28.dp)) {
                        com.mopedtracker.ui.screens.VehiclePickerContent(garage, 2L, {}, {}, {})
                    }
                }
            }
        }
        shot("vehicle_picker")
    }

    @Test fun homeWithVehicles() {
        rule.setContent {
            MopedTheme(true) {
                HomeScreen(vehicleRides, idle, SpeedUnit.KMH, false, {}, {}, {}, {}, vehicles = garage)
            }
        }
        shot("home_vehicles")
    }

    @Test fun homeFilteredByVehicle() {
        rule.setContent {
            MopedTheme(true) {
                HomeScreen(vehicleRides, idle, SpeedUnit.KMH, false, {}, {}, {}, {}, vehicles = garage, vehicleFilter = 2L)
            }
        }
        shot("home_vehicle_filter")
    }

    @Test @Config(sdk = [35], qualifiers = "w411dp-h2400dp-xxhdpi")
    fun detailWithVehicle() {
        val (_, p) = rideData[0]
        val s = vehicleRides[0]
        rule.setContent { MopedTheme(true) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.KMH, {}, {}, {}, vehicle = garage[0]) } }
        shot("detail_vehicle")
    }

    @Test fun fuelWithVehicles() {
        val withVehicles = fuel.mapIndexed { i, e -> e.copy(vehicleId = if (i == 1) 2L else 1L) }
        val stats = FuelCalc.compute(withVehicles, vehicleRides)
        rule.setContent { MopedTheme(true) { FuelScreen(withVehicles, stats, SpeedUnit.KMH, {}, {}, {}, vehicles = garage, vehicleFilter = null) } }
        shot("fuel_vehicles")
    }

    @Test fun shareCardWithVehicleName() {
        val (_, p) = rideData[0]
        val bmp = ShareCard.render(vehicleRides[0], p, SpeedUnit.KMH, "Vespa GTS 125")
        File("build/screenshots/share_card_vehicle.png").apply { parentFile?.mkdirs() }.outputStream().use {
            bmp.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
    }

    // endregion
}
