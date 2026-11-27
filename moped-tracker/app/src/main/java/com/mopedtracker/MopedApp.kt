package com.mopedtracker

import android.app.Application
import androidx.room.Room
import com.mopedtracker.data.RideDatabase
import com.mopedtracker.data.RideRepository
import com.mopedtracker.ui.Settings

class MopedApp : Application() {
    val database: RideDatabase by lazy {
        Room.databaseBuilder(this, RideDatabase::class.java, "rides.db").build()
    }
    val repository: RideRepository by lazy { RideRepository(database.dao()) }
    val settings: Settings by lazy { Settings(this) }
}
