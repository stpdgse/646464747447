package com.kamikazeskies.net;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.PilotRegistry;

import net.minecraft.server.level.ServerPlayer;
import net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent;
import net.neoforged.neoforge.network.registration.PayloadRegistrar;

/** Payload registration and server-side handlers. */
public final class KsNetwork {
    public static final String PROTOCOL = "1";

    private KsNetwork() {}

    public static void register(RegisterPayloadHandlersEvent event) {
        PayloadRegistrar r = event.registrar(PROTOCOL);
        r.playToServer(PilotInputPayload.TYPE, PilotInputPayload.CODEC, (payload, ctx) -> ctx.enqueueWork(() -> {
            if (!(ctx.player() instanceof ServerPlayer sp)) return;
            DroneEntity drone = PilotRegistry.of(sp);
            if (drone != null) drone.acceptPilotInput(sp, payload.toInput());
        }));
        r.playToServer(ExitPilotPayload.TYPE, ExitPilotPayload.CODEC, (payload, ctx) -> ctx.enqueueWork(() -> {
            if (ctx.player() instanceof ServerPlayer sp) PilotRegistry.releasePlayer(sp);
        }));
    }
}
