package com.openfpv.net;

import com.openfpv.server.ViewerManager;
import io.netty.buffer.ByteBuf;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Client -> server: start or stop piloting a drone through its camera. */
public class PacketView implements IMessage {
    public int entityId;
    public boolean start;

    public PacketView() {}

    public PacketView(int entityId, boolean start) {
        this.entityId = entityId;
        this.start = start;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        entityId = buf.readInt();
        start = buf.readBoolean();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeInt(entityId);
        buf.writeBoolean(start);
    }

    public static class Handler implements IMessageHandler<PacketView, IMessage> {
        @Override
        public IMessage onMessage(PacketView msg, MessageContext ctx) {
            EntityPlayerMP player = ctx.getServerHandler().player;
            player.getServerWorld().addScheduledTask(() -> {
                if (msg.start) ViewerManager.begin(player, msg.entityId);
                else ViewerManager.end(player, -1);
            });
            return null;
        }
    }
}
