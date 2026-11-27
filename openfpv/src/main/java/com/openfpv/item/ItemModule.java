package com.openfpv.item;

import java.util.List;
import javax.annotation.Nullable;
import net.minecraft.client.resources.I18n;
import net.minecraft.client.util.ITooltipFlag;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.world.World;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/** A camera module. It does nothing on its own: put it in a Camera Upgrade's module bay. */
public class ItemModule extends Item {
    public final ModuleType type;

    public ItemModule(ModuleType type) {
        this.type = type;
        setMaxStackSize(1);
    }

    @Override
    @SideOnly(Side.CLIENT)
    public void addInformation(ItemStack stack, @Nullable World world, List<String> tooltip, ITooltipFlag flag) {
        tooltip.add(I18n.format("tooltip.openfpv." + type.itemName));
        tooltip.add(I18n.format("tooltip.openfpv.module_hint"));
    }
}
