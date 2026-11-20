package com.kamikazeskies.net;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.resources.Identifier;

/** Client to server: the pilot's current stick/switch state. Sent every client tick while piloting. */
public record PilotInputPayload(float throttle, float roll, float pitch, float yaw, boolean armed, float assist)
        implements CustomPacketPayload {
    public static final Type<PilotInputPayload> TYPE =
            new Type<>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "pilot_input"));

    public static final StreamCodec<RegistryFriendlyByteBuf, PilotInputPayload> CODEC = StreamCodec.composite(
            ByteBufCodecs.FLOAT, PilotInputPayload::throttle,
            ByteBufCodecs.FLOAT, PilotInputPayload::roll,
            ByteBufCodecs.FLOAT, PilotInputPayload::pitch,
            ByteBufCodecs.FLOAT, PilotInputPayload::yaw,
            ByteBufCodecs.BOOL, PilotInputPayload::armed,
            ByteBufCodecs.FLOAT, PilotInputPayload::assist,
            PilotInputPayload::new);

    public static PilotInputPayload of(FlightInput in) {
        return new PilotInputPayload((float) in.throttle(), (float) in.roll(), (float) in.pitch(), (float) in.yaw(),
                in.armed(), (float) in.assist());
    }

    /** Values are re-clamped (and NaN-sanitized) by {@link FlightInput}'s constructor: never trust the client. */
    public FlightInput toInput() { return new FlightInput(throttle, roll, pitch, yaw, armed, assist); }

    @Override
    public Type<? extends CustomPacketPayload> type() { return TYPE; }
}
