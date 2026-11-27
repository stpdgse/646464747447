package com.openfpv;

import net.minecraft.util.ResourceLocation;
import net.minecraft.util.SoundEvent;
import net.minecraftforge.event.RegistryEvent;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;

/** Sound events (the .ogg files and sounds.json live in assets/openfpv). */
@Mod.EventBusSubscriber(modid = OpenFPV.MOD_ID)
public final class ModSounds {
    private ModSounds() {}

    public static final SoundEvent MOTOR = event("drone_motor");
    public static final SoundEvent LOW_BATTERY = event("beep_low_battery");
    public static final SoundEvent LINK_UP = event("link_up");
    public static final SoundEvent LINK_LOST = event("link_lost");
    public static final SoundEvent REC_CLICK = event("rec_click");

    private static SoundEvent event(String name) {
        ResourceLocation rl = new ResourceLocation(OpenFPV.MOD_ID, name);
        return new SoundEvent(rl).setRegistryName(rl);
    }

    @SubscribeEvent
    public static void register(RegistryEvent.Register<SoundEvent> e) {
        e.getRegistry().registerAll(MOTOR, LOW_BATTERY, LINK_UP, LINK_LOST, REC_CLICK);
    }
}
