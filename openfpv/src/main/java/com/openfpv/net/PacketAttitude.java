package com.openfpv.net;

import com.openfpv.client.FpvClient;
import io.netty.buffer.ByteBuf;
import net.minecraft.client.Minecraft;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Server -> pilot, every tick: the piloted drone's attitude in degrees (yaw, nose-down pitch, right roll). */
public class PacketAttitude implements IMessage {
    public int entityId;
    public float yaw, pitch, roll;

    public PacketAttitude() {}

    public PacketAttitude(int entityId, float yaw, float pitch, float roll) {
        this.entityId = entityId;
        this.yaw = yaw;
        this.pitch = pitch;
        this.roll = roll;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        entityId = buf.readInt();
        yaw = buf.readFloat();
        pitch = buf.readFloat();
        roll = buf.readFloat();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeInt(entityId);
        buf.writeFloat(yaw);
        buf.writeFloat(pitch);
        buf.writeFloat(roll);
    }

    public static class Handler implements IMessageHandler<PacketAttitude, IMessage> {
        @Override
        public IMessage onMessage(PacketAttitude msg, MessageContext ctx) {
            Minecraft.getMinecraft().addScheduledTask(() -> FpvClient.onAttitude(msg));
            return null;
        }
    }
}
