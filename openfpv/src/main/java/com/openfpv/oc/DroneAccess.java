package com.openfpv.oc;

import com.openfpv.OpenFPV;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import li.cil.oc.api.internal.Drone;
import net.minecraft.entity.Entity;
import net.minecraft.network.datasync.DataParameter;
import net.minecraft.util.math.Vec3d;

/**
 * Reaches the few drone internals that OpenComputers' public API does not expose.
 *
 * <ul>
 * <li>{@link #setTarget}: the API's setTarget() rounds the target to quarter blocks, which is far too
 *     coarse to steer by small per-tick accelerations, so the synced target data parameters are written
 *     directly.</li>
 * <li>{@link #limits}/{@link #setLimits}: speed cap and drag, raised while somebody is piloting.</li>
 * </ul>
 * Everything is looked up reflectively by name once; if OpenComputers ever renames these, the code
 * falls back to the public API (coarser flight) instead of crashing.
 */
public final class DroneAccess {
    private DroneAccess() {}

    private static boolean tried;
    private static boolean exactTargets;
    private static boolean limitsOk;
    private static Method dataX, dataY, dataZ;
    private static Field maxVelocity, drag;

    private static synchronized void init(Entity sample) {
        if (tried) return;
        tried = true;
        Class<?> c = sample.getClass();
        try {
            dataX = c.getMethod("DataTargetX");
            dataY = c.getMethod("DataTargetY");
            dataZ = c.getMethod("DataTargetZ");
            exactTargets = true;
        } catch (Exception ex) {
            OpenFPV.log.warn("OpenComputers drone target parameters not found ({}); flight will be coarse", ex.toString());
        }
        try {
            maxVelocity = c.getDeclaredField("maxVelocity");
            drag = c.getDeclaredField("drag");
            maxVelocity.setAccessible(true);
            drag.setAccessible(true);
            limitsOk = true;
        } catch (Exception ex) {
            OpenFPV.log.warn("OpenComputers drone limits not found ({}); speed cap stays at the OC default", ex.toString());
        }
    }

    /** Sets the drone's movement target to an exact world position. */
    @SuppressWarnings("unchecked")
    public static void setTarget(Entity drone, double x, double y, double z) {
        init(drone);
        if (exactTargets) {
            try {
                drone.getDataManager().set((DataParameter<Float>) dataX.invoke(null), (float) x);
                drone.getDataManager().set((DataParameter<Float>) dataY.invoke(null), (float) y);
                drone.getDataManager().set((DataParameter<Float>) dataZ.invoke(null), (float) z);
                return;
            } catch (Exception ex) {
                exactTargets = false;
                OpenFPV.log.warn("writing exact drone targets failed ({}); falling back to the OC API", ex.toString());
            }
        }
        if (drone instanceof Drone) ((Drone) drone).setTarget(new Vec3d(x, y, z));
    }

    /** Current {max velocity, drag}, or null if they cannot be read. */
    public static float[] limits(Entity drone) {
        init(drone);
        if (!limitsOk) return null;
        try {
            return new float[]{maxVelocity.getFloat(drone), drag.getFloat(drone)};
        } catch (Exception ex) {
            return null;
        }
    }

    public static void setLimits(Entity drone, float maxVel, float dragValue) {
        init(drone);
        if (!limitsOk) return;
        try {
            maxVelocity.setFloat(drone, maxVel);
            drag.setFloat(drone, dragValue);
        } catch (Exception ex) {
            OpenFPV.log.warn("could not change drone limits: {}", ex.toString());
        }
    }
}
