package com.mopedtracker.data

import androidx.room.Dao
import androidx.room.Database
import androidx.room.Insert
import androidx.room.Query
import androidx.room.RoomDatabase
import androidx.room.Update
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
}

@Database(entities = [SessionEntity::class, TrackPointEntity::class], version = 1, exportSchema = false)
abstract class RideDatabase : RoomDatabase() {
    abstract fun dao(): RideDao
}
