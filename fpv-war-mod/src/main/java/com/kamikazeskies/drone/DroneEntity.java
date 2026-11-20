package com.kamikazeskies.drone;

import java.util.function.IntFunction;

import org.jspecify.annotations.Nullable;

import com.geckolib.animatable.GeoEntity;
import com.geckolib.animatable.instance.AnimatableInstanceCache;
import com.geckolib.animatable.manager.AnimatableManager;
import com.geckolib.animation.AnimationController;
import com.geckolib.animation.RawAnimation;
import com.geckolib.animation.object.PlayState;
import com.geckolib.util.GeckoLibUtil;
import com.kamikazeskies.core.KsConfig;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;
import com.kamikazeskies.sim.FlightSim;
import com.kamikazeskies.sim.Quat;
import com.kamikazeskies.sim.Vec3d;

import net.minecraft.network.chat.Component;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;

/**
 * A player-piloted quadcopter. The server owns the physics ({@link FlightSim}, substepped each tick, with real
 * block collisions); the client only renders, using the synced orientation.
 */
public class DroneEntity extends Entity implements GeoEntity {
    private static final EntityDataAccessor<Float> QW = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> QX = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> QY = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> QZ = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> MOTOR = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> SPEED = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Boolean> ARMED = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.BOOLEAN);
    private static final EntityDataAccessor<Integer> PILOT = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.INT);

    /** Ticks without pilot input after which the link is considered lost (failsafe: motors off). */
    private static final int INPUT_TIMEOUT_TICKS = 10;
    private static final double GROUND_FRICTION = 0.9;
    /** Sentinels far from overflow range: {@code now - lastInputTick} must never wrap. */
    private static final long NEVER = -1_000_000L, ALWAYS = Long.MAX_VALUE / 2;

    private static final RawAnimation SPIN = RawAnimation.begin().thenLoop("animation.fpv_drone.spin");
    private final AnimatableInstanceCache animCache = GeckoLibUtil.createInstanceCache(this);

    // server side
    private final FlightSim sim = new FlightSim(DroneParams.fiveInch());
    private FlightInput input = FlightInput.DISARMED;
    private long lastInputTick = NEVER;
    private @Nullable ServerPlayer pilot;
    private boolean simInitialised;
    private int serverTicks;
    private @Nullable IntFunction<FlightInput> script;   // dev/test: scripted flight, ignores pilot packets

    // client side render interpolation
    private Quat prevQuat = Quat.IDENTITY;
    private Quat currQuat = Quat.IDENTITY;
    private float motorSpin;       // accumulated prop angle, radians
    private float prevMotorSpin;

    public DroneEntity(EntityType<? extends DroneEntity> type, Level level) {
        super(type, level);
        this.setNoGravity(true);   // the flight sim applies its own gravity
    }

    // ---- GeckoLib -----------------------------------------------------------------------------------
    @Override
    public void registerControllers(AnimatableManager.ControllerRegistrar controllers) {
        // props spin while armed; faster with more motor output
        controllers.add(new AnimationController<DroneEntity>("props", 0, test -> {
            DroneEntity d = test.animatable();
            if (!d.isArmed()) return PlayState.STOP;
            test.setControllerSpeed(1.0f + 5.0f * d.motorFraction());
            return test.setAndContinue(SPIN);
        }));
    }

    @Override
    public AnimatableInstanceCache getAnimatableInstanceCache() { return animCache; }

    // ---- synced data --------------------------------------------------------------------------------
    @Override
    protected void defineSynchedData(SynchedEntityData.Builder b) {
        b.define(QW, 1f).define(QX, 0f).define(QY, 0f).define(QZ, 0f)
                .define(MOTOR, 0f).define(SPEED, 0f).define(ARMED, false).define(PILOT, -1);
    }

    public Quat syncedOrientation() {
        return new Quat(entityData.get(QW), entityData.get(QX), entityData.get(QY), entityData.get(QZ)).normalized();
    }

    /** Orientation interpolated between the last two ticks for smooth rendering (client). */
    public Quat renderOrientation(float partialTick) { return Quat.slerp(prevQuat, currQuat, partialTick); }

    public float motorFraction() { return entityData.get(MOTOR); }
    /** True speed in m/s as simulated by the server. */
    public float speed() { return entityData.get(SPEED); }
    public boolean isArmed() { return entityData.get(ARMED); }
    public int pilotEntityId() { return entityData.get(PILOT); }
    public float renderPropAngle(float partialTick) { return prevMotorSpin + (motorSpin - prevMotorSpin) * partialTick; }

    // ---- server-side control API --------------------------------------------------------------------
    /** Set the starting heading and level attitude. */
    public void resetOrientation(float minecraftYawDeg) {
        Quat q = Quat.fromMinecraftYaw(minecraftYawDeg);
        sim.setState(toSim(position()), Vec3d.ZERO, q, Vec3d.ZERO);
        simInitialised = true;
        publishOrientation(q);
    }

    public @Nullable ServerPlayer pilot() { return pilot; }

    public void startPilot(ServerPlayer player) {
        this.pilot = player;
        PilotRegistry.bind(player, this);
        entityData.set(PILOT, player.getId());
        this.input = FlightInput.DISARMED;
        this.lastInputTick = level().getGameTime();
        player.sendOverlayMessage(Component.translatable("message.kamikaze_skies.link_established"));
    }

    public void stopPilot() {
        PilotRegistry.unbind(this.pilot, this);
        this.pilot = null;
        entityData.set(PILOT, -1);
        this.input = FlightInput.DISARMED;
        entityData.set(ARMED, false);
    }

    /** Called by the network layer. Ignored unless {@code player} is the current pilot. */
    public void acceptPilotInput(ServerPlayer player, FlightInput in) {
        if (player != pilot || script != null) return;
        this.input = in;
        this.lastInputTick = level().getGameTime();
    }

    /** Dev/test hook: fly a script (server tick since this call -> input). Pilot packets are ignored while set. */
    public void debugScript(IntFunction<FlightInput> script) { this.script = script; this.scriptStart = serverTicks; this.lastInputTick = ALWAYS; }
    private int scriptStart;

    /** Test hook: drive the drone without a player. */
    public void debugSetInput(FlightInput in) {
        this.input = in;
        this.lastInputTick = ALWAYS;   // never times out
    }

    /** Test hook: make the last input look stale, as if the link had dropped. */
    public void debugExpireInput() { this.lastInputTick = NEVER; }

    /** Test hook: motors already spinning at hover. */
    public void debugSpinUpToHover() { ensureSim(); sim.spinUpToHover(); }

    /** Number of server ticks this drone has simulated (tests wait on this, not on wall-clock ticks). */
    public int simTicks() { return serverTicks; }

    public FlightSim sim() { ensureSim(); return sim; }

    // ---- tick ---------------------------------------------------------------------------------------
    @Override
    public void tick() {
        super.tick();
        if (level().isClientSide()) {
            clientTick();
        } else {
            serverTick();
        }
    }

    private void ensureSim() {
        if (!simInitialised) {
            sim.setState(toSim(position()), Vec3d.ZERO, Quat.fromMinecraftYaw(getYRot()), Vec3d.ZERO);
            simInitialised = true;
        }
    }

    private void serverTick() {
        serverTicks++;
        ensureSim();
        if (pilot != null && (pilot.isRemoved() || !pilot.isAlive() || pilot.level() != level())) stopPilot();

        long now = level().getGameTime();
        FlightInput in = script != null ? script.apply(serverTicks - scriptStart)
                : (lastInputTick != ALWAYS && now - lastInputTick > INPUT_TIMEOUT_TICKS) ? FlightInput.DISARMED : input;

        int substeps = KsConfig.FLIGHT_SUBSTEPS.getAsInt();
        double dt = 1.0 / (20.0 * substeps);
        sim.setPosition(toSim(position()));
        for (int i = 0; i < substeps; i++) {
            Vec3d disp = sim.step(in, dt);
            Vec3 before = position();
            move(MoverType.SELF, new Vec3(disp.x(), disp.y(), disp.z()));
            Vec3 after = position();
            sim.commit(disp, new Vec3d(after.x - before.x, after.y - before.y, after.z - before.z));
            if (onGround()) sim.groundContact(GROUND_FRICTION);
            if (sim.impactSpeed() > KsConfig.CRASH_SPEED.getAsDouble()) {
                crash(sim.impactSpeed());
                return;
            }
        }

        Vec3d v = sim.velocity();
        setDeltaMovement(v.x() / 20.0, v.y() / 20.0, v.z() / 20.0);
        Quat q = sim.orientation();
        setYRot((float) q.minecraftYawDeg());
        setXRot((float) q.minecraftPitchDeg());
        publishOrientation(q);
        entityData.set(MOTOR, (float) Math.round(sim.averageMotorFraction() * 50) / 50f);
        entityData.set(SPEED, (float) Math.round(sim.speed() * 10) / 10f);
        entityData.set(ARMED, in.armed());
    }

    private void clientTick() {
        prevQuat = currQuat;
        currQuat = syncedOrientation();
        prevMotorSpin = motorSpin;
        // prop angle advances with motor output: up to ~2 revolutions per tick at full power (reads as a blur)
        motorSpin += (float) (isArmed() ? (0.6 + 12.0 * motorFraction()) : 0.0);
    }

    private void publishOrientation(Quat q) {
        entityData.set(QW, (float) q.w());
        entityData.set(QX, (float) q.x());
        entityData.set(QY, (float) q.y());
        entityData.set(QZ, (float) q.z());
    }

    private void crash(double impact) {
        if (pilot != null) {
            pilot.sendOverlayMessage(Component.translatable("message.kamikaze_skies.crashed", String.format("%.1f", impact)));
        }
        if (level() instanceof ServerLevel server) {
            server.sendParticles(net.minecraft.core.particles.ParticleTypes.POOF,
                    getX(), getY() + 0.1, getZ(), 12, 0.15, 0.05, 0.15, 0.02);
        }
        stopPilot();
        discard();
    }

    // ---- interaction --------------------------------------------------------------------------------
    @Override
    public InteractionResult interact(Player player, InteractionHand hand, Vec3 location) {
        if (level().isClientSide()) return InteractionResult.SUCCESS;
        if (!(player instanceof ServerPlayer sp)) return InteractionResult.PASS;
        if (player.isShiftKeyDown()) {            // pick the drone up
            stopPilot();
            ItemStack stack = new ItemStack(KsRegistries.DRONE_ITEM.get());
            if (!player.getInventory().add(stack)) spawnAtLocation((ServerLevel) level(), stack);
            discard();
            return InteractionResult.SUCCESS;
        }
        if (pilot == null || pilot == sp) startPilot(sp);
        return InteractionResult.SUCCESS;
    }

    @Override public boolean isPickable() { return true; }
    @Override public boolean canBeCollidedWith(@Nullable Entity other) { return false; }
    @Override public boolean hurtServer(ServerLevel level, DamageSource source, float damage) { return false; }

    @Override
    public void remove(RemovalReason reason) {
        if (!level().isClientSide() && pilot != null) entityData.set(PILOT, -1);
        super.remove(reason);
    }

    // ---- persistence (parked drones keep their pose; flight state is not persisted) -----------------
    @Override protected void readAdditionalSaveData(ValueInput in) { simInitialised = false; }
    @Override protected void addAdditionalSaveData(ValueOutput out) { }

    private static Vec3d toSim(Vec3 v) { return new Vec3d(v.x, v.y, v.z); }
}
