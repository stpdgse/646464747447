package com.openfpv.item;

import com.openfpv.OpenFPV;
import com.openfpv.bay.ModuleBay;
import java.util.List;
import javax.annotation.Nullable;
import net.minecraft.client.resources.I18n;
import net.minecraft.client.util.ITooltipFlag;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.util.ActionResult;
import net.minecraft.util.EnumActionResult;
import net.minecraft.util.EnumHand;
import net.minecraft.world.World;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/**
 * The Camera Upgrade: installs into one upgrade slot of an OpenComputers drone and provides the
 * "camera" component. Right-click to open its module bay.
 */
public class ItemCamera extends Item {
    public ItemCamera() {
        setMaxStackSize(1);
    }

    @Override
    public ActionResult<ItemStack> onItemRightClick(World world, EntityPlayer player, EnumHand hand) {
        ItemStack stack = player.getHeldItem(hand);
        if (!world.isRemote) {
            player.openGui(OpenFPV.instance, com.openfpv.bay.GuiHandler.GUI_MODULE_BAY, world, hand.ordinal(), 0, 0);
        }
        return new ActionResult<>(EnumActionResult.SUCCESS, stack);
    }

    @Override
    @SideOnly(Side.CLIENT)
    public void addInformation(ItemStack stack, @Nullable World world, List<String> tooltip, ITooltipFlag flag) {
        tooltip.add(I18n.format("tooltip.openfpv.camera_upgrade"));
        int mask = ModuleBay.readMask(stack);
        if (mask == 0) {
            tooltip.add(I18n.format("tooltip.openfpv.no_modules"));
        } else {
            tooltip.add(I18n.format("tooltip.openfpv.modules"));
            for (ModuleType t : ModuleType.VALUES) {
                if (ModuleType.has(mask, t)) {
                    tooltip.add("  §a+ §7" + I18n.format("item.openfpv." + t.itemName + ".name"));
                }
            }
        }
        tooltip.add(I18n.format("tooltip.openfpv.open_bay"));
    }
}
