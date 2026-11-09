package com.openfpv.item;

import com.openfpv.Config;
import java.util.List;
import javax.annotation.Nullable;
import net.minecraft.client.resources.I18n;
import net.minecraft.client.util.ITooltipFlag;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.world.World;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/** A long-range antenna (tier 1..4). Goes into the antenna slot of a Camera Upgrade's bay. */
public class ItemAntenna extends Item {
    public final int tier;

    public ItemAntenna(int tier) {
        this.tier = tier;
        setMaxStackSize(1);
    }

    @Override
    @SideOnly(Side.CLIENT)
    public void addInformation(ItemStack stack, @Nullable World world, List<String> tooltip, ITooltipFlag flag) {
        tooltip.add(I18n.format("tooltip.openfpv.antenna", Math.round(Config.rangeForTier(tier))));
        tooltip.add(I18n.format("tooltip.openfpv.antenna_hint"));
    }
}
