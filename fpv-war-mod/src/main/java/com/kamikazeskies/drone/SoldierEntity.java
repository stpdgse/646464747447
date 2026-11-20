package com.kamikazeskies.drone;

import com.geckolib.animatable.GeoEntity;
import com.geckolib.animatable.instance.AnimatableInstanceCache;
import com.geckolib.animatable.manager.AnimatableManager;
import com.geckolib.animation.AnimationController;
import com.geckolib.animation.RawAnimation;
import com.geckolib.util.GeckoLibUtil;

import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/**
 * A rifleman (fictional, unaligned infantry). State-driven animation: the server sets a {@link SoldierAnim.Stance}; the client picks the matching
 * GeckoLib animation, with one-shot triggers for firing and flinching. Behaviour (fear, roles, fire discipline) is layered on top in the AI phase.
 */
public class SoldierEntity extends PathfinderMob implements GeoEntity {
    /** Ticks a corpse stays before it is removed (30 s). */
    public static final int CORPSE_TICKS = 600;
    public static final String CONTROLLER = "main";
    public static final String ANIM_PREFIX = SoldierAnim.PREFIX;

    private static final EntityDataAccessor<Byte> STANCE = SynchedEntityData.defineId(SoldierEntity.class, EntityDataSerializers.BYTE);
    private static final EntityDataAccessor<Boolean> DEATH_FORWARD = SynchedEntityData.defineId(SoldierEntity.class, EntityDataSerializers.BOOLEAN);

    private static RawAnimation play(String name) { return RawAnimation.begin().thenPlay(ANIM_PREFIX + name); }
    private static RawAnimation loop(String name) { return RawAnimation.begin().thenLoop(ANIM_PREFIX + name); }
    private static final RawAnimation FIRE = play("fire"), FLINCH = play("flinch"), RELOAD_ANIM = play("reload"),
            IDLE = loop("idle"), WALK = loop("walk"), RUN = loop("run"), AIM_IN = RawAnimation.begin().thenPlay(ANIM_PREFIX + "aim").thenLoop(ANIM_PREFIX + "aim_idle"),
            CROUCH = RawAnimation.begin().thenPlay(ANIM_PREFIX + "crouch").thenLoop(ANIM_PREFIX + "crouch_idle"), PANIC = loop("panic"), FLEE = loop("flee"),
            DEATH_A = RawAnimation.begin().thenPlayAndHold(ANIM_PREFIX + "death_a"), DEATH_B = RawAnimation.begin().thenPlayAndHold(ANIM_PREFIX + "death_b");

    private final AnimatableInstanceCache cache = GeckoLibUtil.createInstanceCache(this);

    public SoldierEntity(EntityType<? extends SoldierEntity> type, Level level) { super(type, level); }

    public static AttributeSupplier.Builder attributes() {
        return PathfinderMob.createMobAttributes().add(Attributes.MAX_HEALTH, 20.0).add(Attributes.MOVEMENT_SPEED, 0.23).add(Attributes.FOLLOW_RANGE, 48.0);
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder b) {
        super.defineSynchedData(b);
        b.define(STANCE, (byte) 0);
        b.define(DEATH_FORWARD, false);
    }

    public SoldierAnim.Stance stance() { return SoldierAnim.Stance.of(entityData.get(STANCE)); }
    public void setStance(SoldierAnim.Stance s) { entityData.set(STANCE, (byte) s.ordinal()); }
    public boolean fellForward() { return entityData.get(DEATH_FORWARD); }

    /** One rifle shot: recoil animation only (sound, muzzle flash, projectile come from the combat layer). */
    public void fireShot() { if (!level().isClientSide() && isAlive()) triggerAnim(CONTROLLER, "fire"); }

    @Override public boolean hurtServer(ServerLevel level, DamageSource source, float amount) {
        boolean hurt = super.hurtServer(level, source, amount);
        if (hurt && isAlive() && amount > 0.5f) triggerAnim(CONTROLLER, "flinch");
        return hurt;
    }

    @Override public void die(DamageSource source) {
        // the fall direction follows where the hit came from: shot from the front falls back, from behind falls forward
        Entity attacker = source.getEntity() != null ? source.getEntity() : source.getDirectEntity();
        boolean forward = false;
        if (attacker != null) {
            double dx = attacker.getX() - getX(), dz = attacker.getZ() - getZ();
            double yaw = Math.toRadians(getYRot());
            double fx = -Math.sin(yaw), fz = Math.cos(yaw);                     // facing vector
            forward = dx * fx + dz * fz < 0;                                    // attacker behind him
        } else forward = random.nextBoolean();
        entityData.set(DEATH_FORWARD, forward);
        super.die(source);
    }

    /** Keep the body for {@link #CORPSE_TICKS} instead of vanishing after one second, so the death animation can finish and play out. */
    @Override protected void tickDeath() {
        deathTime++;
        if (deathTime >= CORPSE_TICKS && !level().isClientSide() && !isRemoved()) remove(Entity.RemovalReason.KILLED);
    }

    @Override public boolean isPushable() { return isAlive() && super.isPushable(); }

    @Override protected void readAdditionalSaveData(ValueInput in) {
        super.readAdditionalSaveData(in);
        setStance(SoldierAnim.Stance.of(in.getIntOr("stance", 0)));
    }
    @Override protected void addAdditionalSaveData(ValueOutput out) {
        super.addAdditionalSaveData(out);
        out.putInt("stance", stance().ordinal());
    }

    @Override public void registerControllers(AnimatableManager.ControllerRegistrar controllers) {
        controllers.add(new AnimationController<SoldierEntity>(CONTROLLER, 3, state -> {
            SoldierEntity s = state.animatable();
            double speed = Math.hypot(s.getX() - s.xo, s.getZ() - s.zo);          // blocks per tick; remote entities report no delta movement on the client
            String a = SoldierAnim.steady(s.isDeadOrDying(), s.fellForward(), s.stance(), speed);
            return state.setAndContinue(switch (a) {
                case "death_a" -> DEATH_A; case "death_b" -> DEATH_B; case "flee" -> FLEE; case "panic" -> PANIC; case "crouch" -> CROUCH;
                case "reload" -> RELOAD_ANIM; case "aim" -> AIM_IN; case "run" -> RUN; case "walk" -> WALK; default -> IDLE; });
        }).triggerableAnim("fire", FIRE).triggerableAnim("flinch", FLINCH));
    }

    @Override public AnimatableInstanceCache getAnimatableInstanceCache() { return cache; }
}
