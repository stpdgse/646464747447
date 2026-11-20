package com.kamikazeskies.drone;

import org.jspecify.annotations.Nullable;

import net.minecraft.world.entity.Mob;

/** Sensing helpers shared by the soldier and tank AI. */
public final class Threats {
    private Threats() {}

    /** The nearest armed drone the mob can see within {@code range}. A parked, disarmed drone is not a threat. */
    public static @Nullable DroneEntity nearestArmedDrone(Mob viewer, double range) {
        DroneEntity best = null; double bestD = range * range;
        for (DroneEntity d : viewer.level().getEntitiesOfClass(DroneEntity.class, viewer.getBoundingBox().inflate(range))) {
            if (!d.isArmed()) continue;
            double dd = viewer.distanceToSqr(d);
            if (dd < bestD && viewer.hasLineOfSight(d)) { best = d; bestD = dd; }
        }
        return best;
    }
}
