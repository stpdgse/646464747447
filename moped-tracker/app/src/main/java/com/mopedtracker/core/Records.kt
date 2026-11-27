package com.mopedtracker.core

import com.mopedtracker.data.SessionEntity

enum class RecordKind(val title: String) {
    TOP_SPEED("Top speed"),
    LONGEST_DISTANCE("Longest ride"),
    LONGEST_DURATION("Longest time"),
    FASTEST_AVERAGE("Fastest average"),
    MOST_CLIMB("Most climbing"),
}

/** [value] is in base units: m/s, metres, milliseconds, m/s, metres. */
data class RideRecord(val kind: RecordKind, val value: Double, val session: SessionEntity)

object Records {
    /** Rides with less than this much moving time are ignored for "fastest average" (a short sprint isn't an average). */
    private const val MIN_MOVING_MS_FOR_AVERAGE = 60_000L

    fun compute(sessions: List<SessionEntity>): List<RideRecord> {
        fun best(kind: RecordKind, eligible: (SessionEntity) -> Boolean = { true }, value: (SessionEntity) -> Double): RideRecord? =
            sessions.filter(eligible).maxByOrNull(value)?.takeIf { value(it) > 0 }?.let { RideRecord(kind, value(it), it) }

        return listOfNotNull(
            best(RecordKind.TOP_SPEED) { it.maxSpeedMps.toDouble() },
            best(RecordKind.LONGEST_DISTANCE) { it.distanceM },
            best(RecordKind.LONGEST_DURATION) { it.elapsedMs.toDouble() },
            best(RecordKind.FASTEST_AVERAGE, { it.movingMs >= MIN_MOVING_MS_FOR_AVERAGE }) { it.avgMovingSpeedMps.toDouble() },
            best(RecordKind.MOST_CLIMB) { it.elevationGainM },
        )
    }
}
