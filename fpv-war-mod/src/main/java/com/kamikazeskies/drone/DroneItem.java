package com.kamikazeskies.drone;

import com.kamikazeskies.core.KsRegistries;

import net.minecraft.core.BlockPos;
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
        server.addFreshEntity(drone);
        context.getItemInHand().shrink(1);
        return InteractionResult.SUCCESS;
    }
}
