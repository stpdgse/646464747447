package com.mopedtracker.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

val Orange = Color(0xFFFF8A3D)
val Teal = Color(0xFF3DDBD9)
val GoodGreen = Color(0xFF4ADE80)
val WarnAmber = Color(0xFFFFC145)
val BadRed = Color(0xFFFF5C5C)

private val Dark = darkColorScheme(
    primary = Orange, onPrimary = Color(0xFF1A0F05),
    secondary = Teal, onSecondary = Color(0xFF00201F),
    background = Color(0xFF0E1013), onBackground = Color(0xFFE8EAED),
    surface = Color(0xFF171A1F), onSurface = Color(0xFFE8EAED),
    surfaceVariant = Color(0xFF232830), onSurfaceVariant = Color(0xFF9AA3AE),
    error = BadRed, onError = Color(0xFF2B0000),
    outline = Color(0xFF3A414B),
)

private val Light = lightColorScheme(
    primary = Color(0xFFD9600F), onPrimary = Color.White,
    secondary = Color(0xFF00867F), onSecondary = Color.White,
    background = Color(0xFFF4F5F7), onBackground = Color(0xFF14171B),
    surface = Color.White, onSurface = Color(0xFF14171B),
    surfaceVariant = Color(0xFFE6E9ED), onSurfaceVariant = Color(0xFF5A6470),
    error = Color(0xFFD32F2F), onError = Color.White,
    outline = Color(0xFFC4CAD2),
)

private val AppTypography = Typography(
    displayLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.Bold, fontSize = 96.sp, letterSpacing = (-2).sp),
    headlineLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.Bold, fontSize = 32.sp),
    headlineMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.SemiBold, fontSize = 26.sp),
    titleLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.SemiBold, fontSize = 20.sp),
    titleMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.SemiBold, fontSize = 16.sp),
    bodyLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 16.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 14.sp),
    labelLarge = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.Medium, fontSize = 14.sp),
    labelMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontWeight = FontWeight.Medium, fontSize = 12.sp, letterSpacing = 0.6.sp),
)

@Composable
fun MopedTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (darkTheme) Dark else Light, typography = AppTypography) {
        // Surface supplies the default content colour; without it text outside cards would be black.
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background,
            contentColor = MaterialTheme.colorScheme.onBackground,
            content = content,
        )
    }
}

/** Maps 0..1 to a blue -> green -> yellow -> red ramp used for speed colouring. */
fun speedColor(fraction: Float): Color {
    val f = fraction.coerceIn(0f, 1f)
    return Color.hsv(hue = 210f * (1f - f), saturation = 0.85f, value = 1f)
}
