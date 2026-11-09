package com.openfpv.bay;

import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.util.EnumHand;
import net.minecraft.world.World;
import net.minecraftforge.fml.common.network.IGuiHandler;

public class GuiHandler implements IGuiHandler {
    public static final int GUI_MODULE_BAY = 1;

    private static EnumHand hand(int x) {
        return x == EnumHand.OFF_HAND.ordinal() ? EnumHand.OFF_HAND : EnumHand.MAIN_HAND;
    }

    @Override
    public Object getServerGuiElement(int id, EntityPlayer player, World world, int x, int y, int z) {
        if (id == GUI_MODULE_BAY) return new ContainerModuleBay(player.inventory, hand(x));
        return null;
    }

    @Override
    public Object getClientGuiElement(int id, EntityPlayer player, World world, int x, int y, int z) {
        if (id == GUI_MODULE_BAY) return new GuiModuleBay(new ContainerModuleBay(player.inventory, hand(x)));
        return null;
    }
}
