package com.kamikazeskies.drone;

import com.geckolib.animatable.GeoEntity;
import com.geckolib.animatable.instance.AnimatableInstanceCache;
import com.geckolib.animatable.manager.AnimatableManager;
import com.geckolib.animation.AnimationController;
import com.geckolib.animation.RawAnimation;
import com.geckolib.animation.object.PlayState;
import com.geckolib.util.GeckoLibUtil;

import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/**
 * A main battle tank (fictional, unmarked). Heavy hit points; when destroyed the turret is thrown clear and the hull stays as a burnt-out wreck.
 * Driving, turret scanning and the cannon recoil are animated; targeting and fire control come with the AI phase.
 */
public class TankEntity extends PathfinderMob implements GeoEntity {
    public static final float MAX_HEALTH = 120f;
    /** A wreck stays for five minutes. */
    public static final int WRECK_TICKS = 6000;
    public static final int FIRE_INTERVAL = 80;
    public static final double SIGHT = 64.0, TRAVERSE_DEG_PER_TICK = 3.0;
    private int aiTick, nextFire;
    private @org.jspecify.annotations.Nullable DroneEntity target;

    public static final String HULL = "hull", WEAPON = "weapon";

    private static final EntityDataAccessor<Boolean> SCANNING = SynchedEntityData.defineId(TankEntity.class, EntityDataSerializers.BOOLEAN);
    private static RawAnimation loop(String n) { return RawAnimation.begin().thenLoop(TankAnim.PREFIX + n); }
    private static final RawAnimation IDLE = loop("idle"), DRIVE = loop("drive"), SCAN = loop("turret_scan"),
            FIRE = RawAnimation.begin().thenPlay(TankAnim.PREFIX + "fire"), DESTROYED = RawAnimation.begin().thenPlayAndHold(TankAnim.PREFIX + "destroyed");

    private final AnimatableInstanceCache cache = GeckoLibUtil.createInstanceCache(this);

    public TankEntity(EntityType<? extends TankEntity> type, Level level) { super(type, level); }

    public static AttributeSupplier.Builder attributes() {
        return PathfinderMob.createMobAttributes().add(Attributes.MAX_HEALTH, MAX_HEALTH).add(Attributes.MOVEMENT_SPEED, 0.18)
                .add(Attributes.KNOCKBACK_RESISTANCE, 1.0).add(Attributes.FOLLOW_RANGE, 64.0);
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder b) { super.defineSynchedData(b); b.define(SCANNING, false); }
    public int aiTicks() { return aiTick; }
    public boolean scanning() { return entityData.get(SCANNING); }
    public void setScanning(boolean v) { entityData.set(SCANNING, v); }

    /** One cannon round: recoil animation (sound, shell and muzzle flash belong to the combat layer). */
    public void fireCannon() { if (!level().isClientSide() && isAlive()) triggerAnim(WEAPON, "fire"); }

    @Override public void die(DamageSource source) {
        super.die(source);
        if (!level().isClientSide()) level().explode(this, getX(), getY() + 1.0, getZ(), 2.5f, Level.ExplosionInteraction.NONE);
    }

    @Override protected void tickDeath() {
        deathTime++;
        if (deathTime >= WRECK_TICKS && !level().isClientSide() && !isRemoved()) remove(Entity.RemovalReason.KILLED);
    }

    /** The crew scans until a drone shows up, then traverses the whole hull onto it (the turret is fixed in this model) and fires when aligned. */
    @Override protected void customServerAiStep(net.minecraft.server.level.ServerLevel level) {
        super.customServerAiStep(level);
        aiTick++;
        if (aiTick % 5 == 0) target = Threats.nearestArmedDrone(this, SIGHT);
        DroneEntity d = target;
        if (d == null || d.isRemoved()) { setScanning(true); return; }
        setScanning(false);
        double dx = d.getX() - getX(), dz = d.getZ() - getZ();
        float want = (float) (Math.toDegrees(Math.atan2(-dx, dz)));
        float diff = net.minecraft.util.Mth.wrapDegrees(want - getYRot());
        float step = (float) Math.max(-TRAVERSE_DEG_PER_TICK, Math.min(TRAVERSE_DEG_PER_TICK, diff));
        setYRot(getYRot() + step); setYBodyRot(getYRot()); setYHeadRot(getYRot());
        if (Math.abs(diff) < 12 && aiTick >= nextFire) {
            nextFire = aiTick + FIRE_INTERVAL;
            fireCannon();
            level.playSound(null, getX(), getY(), getZ(), net.minecraft.sounds.SoundEvents.GENERIC_EXPLODE.value(), net.minecraft.sounds.SoundSource.HOSTILE, 2.0f, 0.55f);
            level.sendParticles(net.minecraft.core.particles.ParticleTypes.LARGE_SMOKE, getX() - Math.sin(Math.toRadians(getYRot())) * 3, getY() + 1.2, getZ() + Math.cos(Math.toRadians(getYRot())) * 3, 6, 0.2, 0.1, 0.2, 0.02);
            if (random.nextDouble() < com.kamikazeskies.ai.Marksmanship.cannonChance(distanceTo(d), d.speed())) d.shotDown();
        }
    }

    @Override public boolean isPushable() { return false; }
    @Override public boolean canBeLeashed() { return false; }

    @Override protected void readAdditionalSaveData(ValueInput in) { super.readAdditionalSaveData(in); setScanning(in.getBooleanOr("scanning", false)); }
    @Override protected void addAdditionalSaveData(ValueOutput out) { super.addAdditionalSaveData(out); out.putBoolean("scanning", scanning()); }

    @Override public void registerControllers(AnimatableManager.ControllerRegistrar controllers) {
        controllers.add(new AnimationController<TankEntity>(HULL, 4, state -> {
            TankEntity t = state.animatable();
            double speed = Math.hypot(t.getX() - t.xo, t.getZ() - t.zo);
            String a = TankAnim.hull(t.isDeadOrDying(), speed);
            return state.setAndContinue(a.equals("destroyed") ? DESTROYED : a.equals("drive") ? DRIVE : IDLE);
        }));
        controllers.add(new AnimationController<TankEntity>(WEAPON, 4, state -> {
            TankEntity t = state.animatable();
            return t.scanning() && !t.isDeadOrDying() ? state.setAndContinue(SCAN) : PlayState.STOP;
        }).triggerableAnim("fire", FIRE));
    }

    @Override public AnimatableInstanceCache getAnimatableInstanceCache() { return cache; }
}
