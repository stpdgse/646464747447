package com.mopedtracker

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.github.takahirom.roborazzi.captureRoboImage
import com.mopedtracker.core.GpsQuality
import com.mopedtracker.core.LatLon
import com.mopedtracker.core.LiveState
import com.mopedtracker.core.RideStats
import com.mopedtracker.core.RideStatus
import com.mopedtracker.ui.SpeedUnit
import com.mopedtracker.ui.screens.DetailScreen
import com.mopedtracker.ui.screens.DetailState
import com.mopedtracker.ui.screens.HomeScreen
import com.mopedtracker.ui.screens.LiveScreen
import com.mopedtracker.ui.theme.MopedTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [35], qualifiers = "w411dp-h891dp-xxhdpi")
class ScreenshotTest {
    @get:Rule val rule = createComposeRule()

    private val rides = listOf(
        SampleData.session(3, "Morning commute", 0.0).first,
        SampleData.session(2, "Sunday cruise", 1.3).first,
        SampleData.session(1, "Ride Nov 20, 2025", 2.1).first,
    )
    private val idle = LiveState()

    private fun shot(name: String) = rule.onRoot().captureRoboImage("build/screenshots/$name.png")

    @Test fun homeDark() {
        rule.setContent { MopedTheme(true) { HomeScreen(rides, idle, SpeedUnit.KMH, {}, {}, {}, {}) } }
        shot("home_dark")
    }

    @Test fun homeEmpty() {
        rule.setContent { MopedTheme(true) { HomeScreen(emptyList(), idle, SpeedUnit.KMH, {}, {}, {}, {}) } }
        shot("home_empty")
    }

    @Test fun homeLight() {
        rule.setContent { MopedTheme(false) { HomeScreen(rides, idle, SpeedUnit.MPH, {}, {}, {}, {}) } }
        shot("home_light_mph")
    }

    private fun liveState(status: RideStatus) = LiveState(
        status = status, sessionId = 9, currentSpeedMps = 12.4f,
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

    @Test fun detailDark() {
        val (s, p) = SampleData.session()
        rule.setContent { MopedTheme(true) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.KMH, {}, {}, {}) } }
        shot("detail_dark")
    }

    @Test fun detailLightMph() {
        val (s, p) = SampleData.session()
        rule.setContent { MopedTheme(false) { DetailScreen(DetailState.Ready(s, p), SpeedUnit.MPH, {}, {}, {}) } }
        shot("detail_light_mph")
    }
}
