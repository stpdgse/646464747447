package com.kamikazeskies.drone;

import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

import org.jspecify.annotations.Nullable;

import net.minecraft.server.level.ServerPlayer;

/** Server-side map of which drone each player is currently piloting (O(1) lookup for incoming input packets). */
public final class PilotRegistry {
    private static final Map<UUID, DroneEntity> PILOTS = new ConcurrentHashMap<>();

    private PilotRegistry() {}

    public static void bind(ServerPlayer player, DroneEntity drone) {
        DroneEntity previous = PILOTS.put(player.getUUID(), drone);
        if (previous != null && previous != drone) previous.stopPilot();   // one drone per pilot
    }

    public static void unbind(@Nullable ServerPlayer player, DroneEntity drone) {
        if (player != null) PILOTS.remove(player.getUUID(), drone);
    }

    public static @Nullable DroneEntity of(ServerPlayer player) { return PILOTS.get(player.getUUID()); }

    public static void releasePlayer(ServerPlayer player) {
        DroneEntity d = PILOTS.remove(player.getUUID());
        if (d != null) d.stopPilot();
    }

    public static void clear() { PILOTS.clear(); }
}
