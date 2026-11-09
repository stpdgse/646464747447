package com.openfpv.net;

import com.openfpv.server.ViewerManager;
import com.openfpv.util.ControlLag;
import io.netty.buffer.ByteBuf;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Client -> server: the pilot's four sticks, each in [-1, 1]. Sent every tick while flying. */
public class PacketControl implements IMessage {
    public int entityId;
    public float pitch, roll, yaw, throttle;

    public PacketControl() {}

    public PacketControl(int entityId, ControlLag.Sticks s) {
        this.entityId = entityId;
        this.pitch = s.pitch;
        this.roll = s.roll;
        this.yaw = s.yaw;
        this.throttle = s.throttle;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        entityId = buf.readInt();
        pitch = buf.readFloat();
        roll = buf.readFloat();
        yaw = buf.readFloat();
        throttle = buf.readFloat();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeInt(entityId);
        buf.writeFloat(pitch);
        buf.writeFloat(roll);
        buf.writeFloat(yaw);
        buf.writeFloat(throttle);
    }

    public static class Handler implements IMessageHandler<PacketControl, IMessage> {
        @Override
        public IMessage onMessage(PacketControl msg, MessageContext ctx) {
            EntityPlayerMP player = ctx.getServerHandler().player;
            player.getServerWorld().addScheduledTask(() -> ViewerManager.control(player, msg));
            return null;
        }
    }
}
