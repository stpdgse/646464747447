package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.ModSounds;
import com.openfpv.util.CameraMath;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.audio.MovingSound;
import net.minecraft.client.audio.PositionedSoundRecord;
import net.minecraft.entity.Entity;
import net.minecraft.util.SoundCategory;
import net.minecraft.util.SoundEvent;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/** Motor hum that follows each powered drone; pitch and volume rise with its speed. Plus UI beeps. */
@SideOnly(Side.CLIENT)
public final class DroneSounds {
    private DroneSounds() {}

    /** The looping hum attached to one drone entity. */
    static final class Hum extends MovingSound {
        final Entity drone;

        Hum(Entity drone) {
            super(ModSounds.MOTOR, SoundCategory.NEUTRAL);
            this.drone = drone;
            this.repeat = true;
            this.repeatDelay = 0;
            this.volume = 0.4f;
            sync();
        }

        private void sync() {
            xPosF = (float) drone.posX;
            yPosF = (float) drone.posY;
            zPosF = (float) drone.posZ;
        }

        @Override
        public void update() {
            if (drone.isDead || !DroneRenderer.running(drone) || !Config.droneSounds) {
                donePlaying = true;
                return;
            }
            sync();
            double speed = Math.sqrt(drone.motionX * drone.motionX + drone.motionY * drone.motionY + drone.motionZ * drone.motionZ);
            float f = (float) CameraMath.clamp(speed * 1.4, 0, 1);
            pitch = 0.8f + f * 0.7f;
            volume = 0.35f + f * 0.4f;
        }
    }

    private static final Map<Integer, Hum> HUMS = new HashMap<>();
    private static Class<?> droneClass;
    private static int lowBatteryTimer;

    /** Number of hums currently playing (for the self test). */
    public static int active() {
        int n = 0;
        for (Hum h : HUMS.values()) if (!h.isDonePlaying()) n++;
        return n;
    }

    public static void clientTick() {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.world == null || mc.player == null || !Config.droneSounds) {
            HUMS.clear();
            return;
        }
        if (droneClass == null) {
            try {
                droneClass = Class.forName("li.cil.oc.common.entity.Drone");
            } catch (ClassNotFoundException e) {
                return;
            }
        }
        if (mc.world.getTotalWorldTime() % 10 == 0) {
            for (Entity e : mc.world.loadedEntityList) {
                if (!droneClass.isInstance(e) || e.isDead || !DroneRenderer.running(e)) continue;
                if (e.getDistanceSq(mc.player) > 48 * 48) continue;
                Hum h = HUMS.get(e.getEntityId());
                if (h == null || h.isDonePlaying() || h.drone != e) {
                    h = new Hum(e);
                    HUMS.put(e.getEntityId(), h);
                    mc.getSoundHandler().playSound(h);
                }
            }
            Iterator<Hum> it = HUMS.values().iterator();
            while (it.hasNext()) {
                Hum h = it.next();
                if (h.isDonePlaying() || h.drone.isDead) it.remove();
            }
        }
        // low battery warning in the pilot's ears
        FpvClient.Cam cam = FpvClient.current();
        if (FpvClient.isFlying() && cam != null && cam.energy < Config.lowBatteryReturn + 0.05 && cam.energy > 0) {
            if (--lowBatteryTimer <= 0) {
                play(ModSounds.LOW_BATTERY);
                lowBatteryTimer = 40;
            }
        } else {
            lowBatteryTimer = 0;
        }
    }

    /** A non-positional UI sound. */
    public static void play(SoundEvent e) {
        if (!Config.droneSounds) return;
        Minecraft.getMinecraft().getSoundHandler().playSound(PositionedSoundRecord.getMasterRecord(e, 1.0f));
    }
}
