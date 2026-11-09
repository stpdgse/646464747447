package com.openfpv.net;

import com.openfpv.client.FpvClient;
import io.netty.buffer.ByteBuf;
import net.minecraft.client.Minecraft;
import net.minecraftforge.fml.common.network.simpleimpl.IMessage;
import net.minecraftforge.fml.common.network.simpleimpl.IMessageHandler;
import net.minecraftforge.fml.common.network.simpleimpl.MessageContext;

/** Server -> client: the FPV session was ended (or refused). */
public class PacketViewEnd implements IMessage {
    public static final int REFUSED = 0;
    public static final int SIGNAL_LOST = 1;
    public static final int DRONE_GONE = 2;
    public static final int CAMERA_OFF = 3;

    public int reason;

    public PacketViewEnd() {}

    public PacketViewEnd(int reason) {
        this.reason = reason;
    }

    @Override
    public void fromBytes(ByteBuf buf) {
        reason = buf.readByte();
    }

    @Override
    public void toBytes(ByteBuf buf) {
        buf.writeByte(reason);
    }

    public static class Handler implements IMessageHandler<PacketViewEnd, IMessage> {
        @Override
        public IMessage onMessage(PacketViewEnd msg, MessageContext ctx) {
            Minecraft.getMinecraft().addScheduledTask(() -> FpvClient.onServerEnded(msg.reason));
            return null;
        }
    }
}
