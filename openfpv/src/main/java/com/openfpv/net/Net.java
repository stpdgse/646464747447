package com.openfpv.net;

import com.openfpv.OpenFPV;
import com.openfpv.oc.CameraEnvironment;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraft.world.World;
import net.minecraftforge.fml.common.network.NetworkRegistry;
import net.minecraftforge.fml.common.network.simpleimpl.SimpleNetworkWrapper;
import net.minecraftforge.fml.relauncher.Side;

public final class Net {
    private Net() {}

    public static final SimpleNetworkWrapper CHANNEL = NetworkRegistry.INSTANCE.newSimpleChannel(OpenFPV.MOD_ID);

    public static void init() {
        CHANNEL.registerMessage(PacketCameraState.Handler.class, PacketCameraState.class, 0, Side.CLIENT);
        CHANNEL.registerMessage(PacketViewEnd.Handler.class, PacketViewEnd.class, 1, Side.CLIENT);
        CHANNEL.registerMessage(PacketView.Handler.class, PacketView.class, 2, Side.SERVER);
        CHANNEL.registerMessage(PacketControl.Handler.class, PacketControl.class, 3, Side.SERVER);
    }

    public static void sendState(CameraEnvironment env, World world) {
        CHANNEL.sendToDimension(PacketCameraState.of(env), world.provider.getDimension());
    }

    public static void sendViewEnd(EntityPlayerMP player, int reason) {
        CHANNEL.sendTo(new PacketViewEnd(reason), player);
    }
}
