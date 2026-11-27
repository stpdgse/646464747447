package com.mopedtracker.data

import androidx.room.Dao
import androidx.room.Database
import androidx.room.Insert
import androidx.room.Query
import androidx.room.RoomDatabase
import androidx.room.Update
import androidx.room.migration.Migration
import androidx.sqlite.db.SupportSQLiteDatabase
import kotlinx.coroutines.flow.Flow

@Dao
interface RideDao {
    @Insert suspend fun insertSession(session: SessionEntity): Long
    @Update suspend fun updateSession(session: SessionEntity)
    @Insert suspend fun insertPoint(point: TrackPointEntity)

    @Query("SELECT * FROM sessions WHERE endTime IS NOT NULL ORDER BY startTime DESC")
    fun observeFinished(): Flow<List<SessionEntity>>

    @Query("SELECT * FROM sessions WHERE id = :id")
    fun observeSession(id: Long): Flow<SessionEntity?>

    @Query("SELECT * FROM sessions WHERE id = :id")
    suspend fun getSession(id: Long): SessionEntity?

    @Query("SELECT * FROM sessions WHERE endTime IS NULL ORDER BY startTime DESC")
    suspend fun getActiveSessions(): List<SessionEntity>

    @Query("SELECT * FROM points WHERE sessionId = :id ORDER BY time")
    suspend fun getPoints(id: Long): List<TrackPointEntity>

    @Query("DELETE FROM sessions WHERE id = :id")
    suspend fun deleteSession(id: Long)

    @Query("UPDATE sessions SET name = :name WHERE id = :id")
    suspend fun rename(id: Long, name: String)

    @Query("UPDATE sessions SET starred = :starred WHERE id = :id")
    suspend fun setStarred(id: Long, starred: Boolean)

    @Insert suspend fun insertFuel(entry: FuelEntryEntity)

    @Query("SELECT * FROM fuel_entries ORDER BY time DESC")
    fun observeFuel(): Flow<List<FuelEntryEntity>>

    @Query("DELETE FROM fuel_entries WHERE id = :id")
    suspend fun deleteFuel(id: Long)
}

@Database(
    entities = [SessionEntity::class, TrackPointEntity::class, FuelEntryEntity::class],
    version = 2,
    exportSchema = false,
)
abstract class RideDatabase : RoomDatabase() {
    abstract fun dao(): RideDao

    companion object {
        /** v1 -> v2: starred rides and the fuel log. Existing rides and points are kept. */
        val MIGRATION_1_2 = object : Migration(1, 2) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL("ALTER TABLE `sessions` ADD COLUMN `starred` INTEGER NOT NULL DEFAULT 0")
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `fuel_entries` (`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, " +
                        "`time` INTEGER NOT NULL, `liters` REAL NOT NULL, `totalCost` REAL NOT NULL, " +
                        "`fullTank` INTEGER NOT NULL, `note` TEXT NOT NULL)",
                )
            }
        }
    }
}
