package com.kamikazeskies.drone;

import com.kamikazeskies.core.KsRegistries;

import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.phys.Vec3;

/** Places a jammer on the clicked block face. */
public class JammerItem extends Item {
    public JammerItem(Properties properties) { super(properties); }

    @Override
    public InteractionResult useOn(UseOnContext context) {
        if (!(context.getLevel() instanceof ServerLevel server)) return InteractionResult.SUCCESS;
        BlockPos pos = context.getClickedPos().relative(context.getClickedFace());
        JammerEntity jammer = KsRegistries.JAMMER.get().create(server, EntitySpawnReason.SPAWN_ITEM_USE);
        if (jammer == null) return InteractionResult.FAIL;
        jammer.snapTo(new Vec3(pos.getX() + 0.5, pos.getY(), pos.getZ() + 0.5));
        server.addFreshEntity(jammer);
        context.getItemInHand().shrink(1);
        return InteractionResult.SUCCESS;
    }
}
