package com.kamikazeskies.drone;

import com.geckolib.animatable.GeoEntity;
import com.geckolib.animatable.instance.AnimatableInstanceCache;
import com.geckolib.animatable.manager.AnimatableManager;
import com.geckolib.animation.AnimationController;
import com.geckolib.animation.RawAnimation;
import com.geckolib.util.GeckoLibUtil;

import com.kamikazeskies.ai.Behaviour;
import com.kamikazeskies.ai.Brain;
import com.kamikazeskies.ai.FearModel;
import com.kamikazeskies.ai.Marksmanship;
import com.kamikazeskies.ai.Role;

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
import net.minecraft.world.entity.ai.util.DefaultRandomPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.world.phys.Vec3;
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

    public static final int MAG_SIZE = 30, RELOAD_TICKS = 56, FIRE_INTERVAL = 10;
    private Role role;
    private FearModel fear;
    private Behaviour behaviour = Behaviour.IDLE;
    private int holdUntil, ammo = MAG_SIZE, reloadUntil, aiTick;
    private double sensedDistance = Double.NaN, sensedClosing, lastDistance = Double.NaN;
    private @org.jspecify.annotations.Nullable DroneEntity seenDrone;
    private @org.jspecify.annotations.Nullable ServerPlayer seenOperator;

    public SoldierEntity(EntityType<? extends SoldierEntity> type, Level level) { super(type, level); setRole(Role.pick(random.nextDouble())); }

    public Role role() { return role; }
    public void setRole(Role r) { role = r; fear = new FearModel(r.courage); }
    public double fear() { return fear.fear(); }
    public void setFear(double f) { fear.set(f); }
    public Behaviour behaviour() { return behaviour; }
    public int ammo() { return ammo; }
    /** Server AI ticks so far (lets tests wait for the entity to actually be ticking). */
    public int aiTicks() { return aiTick; }

    /** A blast of the given size goes off: every soldier within earshot flinches in proportion to how close it was. */
    public static void alarm(net.minecraft.server.level.ServerLevel level, Vec3 center, double blastRadius) {
        double reach = 40.0;
        for (SoldierEntity s : level.getEntitiesOfClass(SoldierEntity.class, new net.minecraft.world.phys.AABB(center, center).inflate(reach))) {
            double d = s.position().distanceTo(center);
            if (d < reach) s.fear.blast(Math.min(1.0, blastRadius / 6.0) * (1.0 - d / reach));
        }
    }

    @Override protected void customServerAiStep(net.minecraft.server.level.ServerLevel level) {
        super.customServerAiStep(level);
        aiTick++;
        if (aiTick % 4 == 0) sense();
        fear.tick(sensedDistance, sensedClosing);
        boolean visible = seenDrone != null || seenOperator != null;
        if (aiTick >= holdUntil) {
            Behaviour next = Brain.decide(fear.fear(), role.courage, visible);
            if (next != behaviour) { behaviour = next; holdUntil = aiTick + Brain.holdTicks(next); if (next == Behaviour.FLEE) reloadUntil = 0; }
        }
        act(level);
    }

    private void sense() {
        DroneEntity d = Threats.nearestArmedDrone(this, FearModel.SIGHT_RANGE);
        seenDrone = d;
        ServerPlayer op = d != null ? d.pilot() : null;
        seenOperator = op != null && distanceTo(op) < 30 && hasLineOfSight(op) ? op : null;
        if (d == null) { sensedDistance = Double.NaN; sensedClosing = 0; lastDistance = Double.NaN; return; }
        sensedDistance = distanceTo(d);
        sensedClosing = Double.isNaN(lastDistance) ? 0 : (lastDistance - sensedDistance) / (4 / 20.0);
        lastDistance = sensedDistance;
    }

    private void act(net.minecraft.server.level.ServerLevel level) {
        if (reloadUntil > aiTick) { setStance(SoldierAnim.Stance.RELOAD); getNavigation().stop(); return; }
        if (reloadUntil != 0) { ammo = MAG_SIZE; reloadUntil = 0; }
        switch (behaviour) {
            case ENGAGE -> {
                getNavigation().stop();
                setStance(SoldierAnim.Stance.AIM);
                Entity target = seenOperator != null && random.nextBoolean() ? seenOperator : seenDrone;
                if (target == null) return;
                getLookControl().setLookAt(target, 40f, 40f);
                if (aiTick % FIRE_INTERVAL == 0) shootAt(level, target);
            }
            case COVER -> {
                getNavigation().stop();
                setStance(SoldierAnim.Stance.CROUCH);
                if (seenDrone != null) getLookControl().setLookAt(seenDrone, 30f, 30f);
            }
            case FLEE -> {
                setStance(SoldierAnim.Stance.PANIC);
                if (getNavigation().isDone() || aiTick % 20 == 0) {
                    Vec3 from = seenDrone != null ? seenDrone.position() : position().add(random.nextDouble() - .5, 0, random.nextDouble() - .5);
                    Vec3 to = DefaultRandomPos.getPosAway(this, 16, 7, from);
                    if (to != null) getNavigation().moveTo(to.x, to.y, to.z, 1.35);
                }
            }
            case IDLE -> {
                setStance(SoldierAnim.Stance.STAND);
                if (getNavigation().isDone() && random.nextInt(120) == 0) {
                    Vec3 to = DefaultRandomPos.getPos(this, 6, 3);
                    if (to != null) getNavigation().moveTo(to.x, to.y, to.z, 0.55);
                }
            }
        }
    }

    private void shootAt(net.minecraft.server.level.ServerLevel level, Entity target) {
        if (ammo <= 0) { reloadUntil = aiTick + RELOAD_TICKS; return; }
        ammo--;
        fireShot();
        level.playSound(null, getX(), getY(), getZ(), SoundEvents.DISPENSER_LAUNCH, SoundSource.HOSTILE, 1.2f, 1.9f);
        Vec3 muzzle = position().add(getLookAngle().scale(0.9)).add(0, 1.35, 0);
        level.sendParticles(ParticleTypes.SMOKE, muzzle.x, muzzle.y, muzzle.z, 2, 0.02, 0.02, 0.02, 0.01);
        double dist = distanceTo(target);
        if (target instanceof DroneEntity d) {
            if (random.nextDouble() < Marksmanship.hitChance(dist, d.speed(), role.skill, fear.fear())) d.shotDown();
        } else if (target instanceof ServerPlayer p) {
            if (random.nextDouble() < Marksmanship.hitChance(dist, 0, role.skill * 0.8, fear.fear())) p.hurtServer(level, level.damageSources().mobAttack(this), 3.0f);
        }
    }

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
        if (level() instanceof net.minecraft.server.level.ServerLevel sl)
            for (SoldierEntity other : sl.getEntitiesOfClass(SoldierEntity.class, getBoundingBox().inflate(24), o -> o != this && o.isAlive() && o.hasLineOfSight(this))) other.fear.allyDied();
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
        try { setRole(Role.valueOf(in.getStringOr("role", role.name()))); } catch (IllegalArgumentException e) { /* keep random role */ }
    }
    @Override protected void addAdditionalSaveData(ValueOutput out) {
        super.addAdditionalSaveData(out);
        out.putInt("stance", stance().ordinal());
        out.putString("role", role.name());
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
