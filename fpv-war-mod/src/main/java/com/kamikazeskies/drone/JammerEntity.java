package com.kamikazeskies.drone;

import java.util.List;

import com.geckolib.animatable.GeoEntity;
import com.geckolib.animatable.instance.AnimatableInstanceCache;
import com.geckolib.animatable.manager.AnimatableManager;
import com.geckolib.util.GeckoLibUtil;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.sim.RadioLink;

import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * An anti-drone jammer: a mast that knocks down the radio link of any drone inside its radius. It can be picked up
 * (sneak + use) and is destroyed by explosions.
 */
public class JammerEntity extends Entity implements GeoEntity {
    /** Radius of the jamming field, in blocks. Full strength in the inner third, fading to nothing at the edge. */
    public static final double RADIUS = 48.0;
    public static final double STRENGTH = 1.0;

    private final AnimatableInstanceCache cache = GeckoLibUtil.createInstanceCache(this);

    public JammerEntity(EntityType<? extends JammerEntity> type, Level level) { super(type, level); }

    /** Combined jamming strength (0..1) of every jammer around a point. */
    public static double jammingAt(Level level, Vec3 pos) {
        List<JammerEntity> near = level.getEntitiesOfClass(JammerEntity.class, new AABB(pos, pos).inflate(RADIUS));
        double total = 0;
        for (JammerEntity j : near) {
            total = RadioLink.combine(total, RadioLink.jammingAt(j.position().distanceTo(pos), RADIUS, STRENGTH));
        }
        return total;
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder b) {}
    @Override protected void readAdditionalSaveData(ValueInput in) {}
    @Override protected void addAdditionalSaveData(ValueOutput out) {}
    @Override public boolean isPickable() { return true; }

    @Override
    public boolean hurtServer(ServerLevel level, DamageSource source, float damage) {
        if (damage >= 4.0f && !isRemoved()) { discard(); return true; }   // explosions and other heavy hits destroy it
        return false;
    }

    @Override
    public InteractionResult interact(Player player, InteractionHand hand, Vec3 location) {
        if (level().isClientSide()) return InteractionResult.SUCCESS;
        if (player.isShiftKeyDown()) {
            ItemStack stack = new ItemStack(KsRegistries.JAMMER_ITEM.get());
            if (!player.getInventory().add(stack)) spawnAtLocation((ServerLevel) level(), stack);
            discard();
            return InteractionResult.SUCCESS;
        }
        return InteractionResult.PASS;
    }

    @Override public void registerControllers(AnimatableManager.ControllerRegistrar controllers) {}
    @Override public AnimatableInstanceCache getAnimatableInstanceCache() { return cache; }
}
