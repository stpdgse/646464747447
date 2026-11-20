package com.kamikazeskies.drone;

import com.kamikazeskies.core.KsRegistries;

import java.util.function.Consumer;

import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.item.KsComponents;

import net.minecraft.ChatFormatting;
import net.minecraft.core.BlockPos;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.TooltipFlag;
import net.minecraft.world.item.component.TooltipDisplay;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;

/** Places a drone entity on the clicked block face. */
public class DroneItem extends Item {
    public DroneItem(Properties properties) { super(properties); }

    @Override
    public void appendHoverText(ItemStack stack, Item.TooltipContext context, TooltipDisplay display, Consumer<Component> out, TooltipFlag flag) {
        DroneBuild b = stack.getOrDefault(KsComponents.DRONE_BUILD.get(), DroneBuild.STANDARD);
        DroneBuild.Stats s = b.stats();
        out.accept(Component.translatable("tooltip.kamikaze_skies.drone.parts", b.frame(), b.motor(), b.prop(), b.battery(), b.flightController(), b.camera(), b.antenna()).withStyle(ChatFormatting.GOLD));
        out.accept(Component.translatable("tooltip.kamikaze_skies.drone.stats", String.format("%.2f", s.massKg()), String.format("%.1f", s.thrustToWeight()), String.format("%.1f", s.hoverMinutes())).withStyle(ChatFormatting.GRAY));
        out.accept(Component.translatable("tooltip.kamikaze_skies.stat.antenna", String.format("%.1f", s.rangeMultiplier())).withStyle(ChatFormatting.GRAY));
        if (b.warhead() != Warhead.NONE) out.accept(Component.translatable("tooltip.kamikaze_skies.drone.warhead." + b.warhead().ordinal()).withStyle(ChatFormatting.RED));
        if (b.blackbox()) out.accept(Component.translatable("tooltip.kamikaze_skies.drone.blackbox").withStyle(ChatFormatting.DARK_GRAY));
    }

    @Override
    public InteractionResult useOn(UseOnContext context) {
        Level level = context.getLevel();
        if (!(level instanceof ServerLevel server)) return InteractionResult.SUCCESS;

        BlockPos pos = context.getClickedPos().relative(context.getClickedFace());
        DroneEntity drone = KsRegistries.DRONE.get().create(server, EntitySpawnReason.SPAWN_ITEM_USE);
        if (drone == null) return InteractionResult.FAIL;
        float yaw = context.getPlayer() != null ? context.getPlayer().getYRot() : 0f;
        drone.snapTo(new Vec3(pos.getX() + 0.5, pos.getY(), pos.getZ() + 0.5));
        drone.setYRot(yaw);
        drone.resetOrientation(yaw);
        drone.setBuild(context.getItemInHand().getOrDefault(com.kamikazeskies.item.KsComponents.DRONE_BUILD.get(), com.kamikazeskies.build.DroneBuild.STANDARD));
        server.addFreshEntity(drone);
        context.getItemInHand().shrink(1);
        return InteractionResult.SUCCESS;
    }
}
