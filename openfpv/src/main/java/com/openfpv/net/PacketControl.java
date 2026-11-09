package com.openfpv.net;

import com.openfpv.server.ViewerManager;
import io.netty.buffer.ByteBuf;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Client -> server: the pilot's desired flight direction (world space, length <= 1). */
public class PacketControl implements IMessage {
    public int entityId;
    public float dx, dy, dz;

    public PacketControl() {}

    public PacketControl(int entityId, double dx, double dy, double dz) {
        this.entityId = entityId;
        this.dx = (float) dx;
        this.dy = (float) dy;
        this.dz = (float) dz;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        entityId = buf.readInt();
        dx = buf.readFloat();
        dy = buf.readFloat();
        dz = buf.readFloat();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeInt(entityId);
        buf.writeFloat(dx);
        buf.writeFloat(dy);
        buf.writeFloat(dz);
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
