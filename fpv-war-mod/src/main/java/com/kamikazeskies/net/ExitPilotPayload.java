package com.kamikazeskies.net;

import com.kamikazeskies.core.KamikazeSkies;

import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.resources.Identifier;

/** Client to server: the pilot wants to take the goggles off. */
public record ExitPilotPayload() implements CustomPacketPayload {
    public static final ExitPilotPayload INSTANCE = new ExitPilotPayload();
    public static final Type<ExitPilotPayload> TYPE =
            new Type<>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "exit_pilot"));
    public static final StreamCodec<RegistryFriendlyByteBuf, ExitPilotPayload> CODEC = StreamCodec.unit(INSTANCE);

    @Override
    public Type<? extends CustomPacketPayload> type() { return TYPE; }
}
