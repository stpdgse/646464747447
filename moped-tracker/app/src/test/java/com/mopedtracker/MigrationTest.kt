package com.mopedtracker

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.mopedtracker.data.FuelEntryEntity
import com.mopedtracker.data.RideDatabase
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.runner.RunWith

/** Builds a real version-1 database (exactly as the first release created it) and upgrades it. */
@RunWith(AndroidJUnit4::class)
class MigrationTest {

    private val v1Tables = listOf(
        "CREATE TABLE IF NOT EXISTS `sessions` (`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, `name` TEXT NOT NULL, `startTime` INTEGER NOT NULL, `endTime` INTEGER, `pausedMs` INTEGER NOT NULL, `distanceM` REAL NOT NULL, `elapsedMs` INTEGER NOT NULL, `movingMs` INTEGER NOT NULL, `maxSpeedMps` REAL NOT NULL, `avgSpeedMps` REAL NOT NULL, `avgMovingSpeedMps` REAL NOT NULL, `elevationGainM` REAL NOT NULL, `elevationLossM` REAL NOT NULL, `maxAltitudeM` REAL, `minAltitudeM` REAL, `avgAccuracyM` REAL NOT NULL, `pointCount` INTEGER NOT NULL, `previewRoute` TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS `points` (`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, `sessionId` INTEGER NOT NULL, `time` INTEGER NOT NULL, `lat` REAL NOT NULL, `lon` REAL NOT NULL, `altitude` REAL, `speedMps` REAL NOT NULL, `accuracyM` REAL NOT NULL, `speedAccuracyMps` REAL, `bearing` REAL, FOREIGN KEY(`sessionId`) REFERENCES `sessions`(`id`) ON UPDATE NO ACTION ON DELETE CASCADE )",
        "CREATE INDEX IF NOT EXISTS `index_points_sessionId` ON `points` (`sessionId`)",
        "CREATE TABLE IF NOT EXISTS room_master_table (id INTEGER PRIMARY KEY,identity_hash TEXT)",
        "INSERT OR REPLACE INTO room_master_table (id,identity_hash) VALUES(42, '6224c715230182c31c3ff35837c216d0')",
    )

    @Test fun upgradeFromV1KeepsRidesAndAddsStarsAndFuel() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val file = context.getDatabasePath("migration-test.db").apply { parentFile?.mkdirs(); delete() }

        SQLiteDatabase.openOrCreateDatabase(file, null).use { db ->
            v1Tables.forEach(db::execSQL)
            db.execSQL(
                "INSERT INTO sessions VALUES (7, 'Old ride', 1000, 9000, 0, 5000.0, 8000, 7000, 12.5, 9.0, 10.0, 20.0, 18.0, 530.0, 500.0, 4.0, 2, '48.1,11.5;48.2,11.6')",
            )
            db.execSQL("INSERT INTO points VALUES (1, 7, 1000, 48.1, 11.5, 510.0, 5.0, 4.0, 0.5, NULL)")
            db.execSQL("INSERT INTO points VALUES (2, 7, 2000, 48.2, 11.6, 511.0, 6.0, 4.0, 0.5, NULL)")
            db.version = 1
        }

        val room = Room.databaseBuilder(context, RideDatabase::class.java, "migration-test.db")
            .addMigrations(RideDatabase.MIGRATION_1_2)
            .allowMainThreadQueries()
            .build()
        try {
            val dao = room.dao()
            runBlocking {
                val old = dao.getSession(7)!!
                assertEquals("Old ride", old.name)
                assertEquals(5000.0, old.distanceM, 0.0)
                assertFalse(old.starred)
                assertEquals(2, dao.getPoints(7).size)

                dao.setStarred(7, true)
                assertEquals(true, dao.getSession(7)!!.starred)

                dao.insertFuel(FuelEntryEntity(time = 5000, liters = 3.2, totalCost = 5.1, fullTank = true, note = "Shell"))
                val fuel = dao.observeFuel().first()
                assertEquals(1, fuel.size)
                assertEquals(3.2, fuel[0].liters, 0.0)
                assertEquals("Shell", fuel[0].note)
            }
        } finally {
            room.close()
        }
    }
}
