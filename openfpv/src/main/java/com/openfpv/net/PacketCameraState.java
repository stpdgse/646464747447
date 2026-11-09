package com.openfpv.net;

import com.openfpv.client.FpvClient;
import com.openfpv.oc.CameraEnvironment;
import io.netty.buffer.ByteBuf;
import net.minecraft.client.Minecraft;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Server -> client: the camera state of one drone. */
public class PacketCameraState implements IMessage {
    public int entityId;
    public boolean active, night, thermal;
    public float fov, tilt, pan, energy;
    public int modules;

    public PacketCameraState() {}

    public static PacketCameraState of(CameraEnvironment e) {
        PacketCameraState p = new PacketCameraState();
        p.entityId = e.entityId();
        p.active = e.isActive();
        p.night = e.isNight();
        p.thermal = e.isThermal();
        p.fov = e.fov();
        p.tilt = e.tilt();
        p.pan = e.pan();
        p.energy = e.energyFraction();
        p.modules = e.modules();
        return p;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        entityId = buf.readInt();
        int flags = buf.readByte();
        active = (flags & 1) != 0;
        night = (flags & 2) != 0;
        thermal = (flags & 4) != 0;
        fov = buf.readFloat();
        tilt = buf.readFloat();
        pan = buf.readFloat();
        energy = buf.readFloat();
        modules = buf.readInt();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeInt(entityId);
        buf.writeByte((active ? 1 : 0) | (night ? 2 : 0) | (thermal ? 4 : 0));
        buf.writeFloat(fov);
        buf.writeFloat(tilt);
        buf.writeFloat(pan);
        buf.writeFloat(energy);
        buf.writeInt(modules);
    }

    public static class Handler implements IMessageHandler<PacketCameraState, IMessage> {
        @Override
        public IMessage onMessage(PacketCameraState msg, MessageContext ctx) {
            Minecraft.getMinecraft().addScheduledTask(() -> FpvClient.onState(msg));
            return null;
        }
    }
}
