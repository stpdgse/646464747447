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
import com.kamikazeskies.destruction.BlastExecutor;
import com.kamikazeskies.sim.Battery;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;
import com.kamikazeskies.sim.FlightSim;
import com.kamikazeskies.sim.LinkMonitor;
import com.kamikazeskies.sim.RadioLink;
import com.kamikazeskies.sim.SignalLevel;
import com.kamikazeskies.sim.Quat;
import com.kamikazeskies.sim.Vec3d;

import net.minecraft.core.BlockPos;
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
    private static final EntityDataAccessor<Integer> WARHEAD = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.INT);
    private static final EntityDataAccessor<Float> VOLTS = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> SOC = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Integer> RSSI = SynchedEntityData.defineId(DroneEntity.class, EntityDataSerializers.INT);
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
    private final Battery battery = new Battery(Battery.Spec.fourS1300());
    private final LinkMonitor link = new LinkMonitor();
    private @Nullable Vec3 debugOperator;
    private FlightInput heldInput = FlightInput.DISARMED;
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
                .define(MOTOR, 0f).define(SPEED, 0f).define(WARHEAD, 0).define(VOLTS, 16.8f).define(SOC, 1f).define(RSSI, 100).define(ARMED, false).define(PILOT, -1);
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
    /** Pack voltage in volts (synced, rounded to 0.1 V). */
    public float voltage() { return entityData.get(VOLTS); }
    /** Battery state of charge, 0..1. */
    public float batterySoc() { return entityData.get(SOC); }
    /** Signal quality, 0..1 (synced as a percentage). */
    public float rssi() { return entityData.get(RSSI) / 100f; }
    public SignalLevel signalLevel() { return SignalLevel.of(rssi()); }
    public boolean batteryLow() { return batterySoc() < 0.12f; }

    public Warhead warhead() { return Warhead.byOrdinal(entityData.get(WARHEAD)); }
    public void setWarhead(Warhead w) { entityData.set(WARHEAD, w.ordinal()); }
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

    /** Test hook: pretend the operator stands here (used when no real pilot is bound). */
    public void debugOperator(Vec3 pos) { this.debugOperator = pos; }
    /** Test hook: set the battery's state of charge. */
    public void debugBatterySoc(double soc) { battery.setSoc(soc); }
    public Battery battery() { return battery; }
    public LinkMonitor link() { return link; }

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

        // --- radio link: failsafe when lost, dropped control packets when weak ---
        updateLink();
        if (link.controlLost()) {
            in = FlightInput.DISARMED;
        } else if (link.dropPacket(level().getRandom().nextDouble())) {
            in = heldInput;                                   // this tick's control packet never arrived
        }
        heldInput = in;

        // --- battery: drain by what the motors drew last tick, and limit power as the loaded voltage collapses ---
        double current = in.armed() ? Battery.currentFor(sim.averageMotorFraction()) : 0.3;
        if (KsConfig.BATTERY_ENABLED.getAsBoolean()) {
            battery.drain(current, 0.05);
            sim.setPowerScale(battery.powerScale(current));
        } else {
            sim.setPowerScale(1.0);
        }

        int substeps = KsConfig.FLIGHT_SUBSTEPS.getAsInt();
        double dt = 1.0 / (20.0 * substeps);
        sim.setPosition(toSim(position()));
        for (int i = 0; i < substeps; i++) {
            Vec3d disp = sim.step(in, dt);
            Vec3 before = position();
            move(MoverType.SELF, new Vec3(disp.x(), disp.y(), disp.z()));
            Vec3 after = position();
            Vec3d preImpactVelocity = sim.velocity();          // sim.commit() removes the blocked components
            sim.commit(disp, new Vec3d(after.x - before.x, after.y - before.y, after.z - before.z));
            if (warhead().armed() && sim.impactSpeed() >= KsConfig.DETONATION_SPEED.getAsDouble()) {
                detonate(preImpactVelocity);
                return;
            }
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
        entityData.set(VOLTS, (float) Math.round(battery.voltage(current) * 10) / 10f);
        entityData.set(SOC, (float) Math.round(battery.soc() * 100) / 100f);
        entityData.set(RSSI, (int) Math.round(link.rssi() * 100));
    }

    /** Evaluate the signal between the operator and the drone: distance, solid blocks on the line, jammers. */
    private void updateLink() {
        Vec3 operator = pilot != null ? pilot.getEyePosition() : debugOperator;
        if (operator == null) { link.update(1.0); return; }          // parked / no operator: nothing to lose
        Vec3 me = position().add(0, getBbHeight() * 0.5, 0);
        double dist = operator.distanceTo(me);
        double blocked = blockedMetres(level(), operator, me);
        double jam = JammerEntity.jammingAt(level(), me);
        link.update(RadioLink.rssi(dist, KsConfig.LINK_RANGE.getAsDouble(), blocked, jam));
    }

    /** Metres of opaque blocks along the straight line between two points (sampled; at most 128 samples). */
    static double blockedMetres(Level level, Vec3 a, Vec3 b) {
        double dist = a.distanceTo(b);
        if (dist < 1.5) return 0;
        int n = (int) Math.min(128, Math.ceil(dist));
        double step = dist / n;
        int blocked = 0;
        BlockPos.MutableBlockPos mp = new BlockPos.MutableBlockPos();
        for (int i = 1; i < n; i++) {
            double t = (double) i / n;
            mp.set(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t);
            if (level.getBlockState(mp).canOcclude()) blocked++;
        }
        return blocked * step;
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

    private void detonate(Vec3d preImpactVelocity) {
        Vec3d dir = preImpactVelocity.normalized();
        Vec3 axis = dir.lengthSq() < 1e-9 ? new Vec3(0, -1, 0) : new Vec3(dir.x(), dir.y(), dir.z());
        if (pilot != null) pilot.sendOverlayMessage(Component.translatable("message.kamikaze_skies.detonated"));
        Vec3 center = position().add(0, getBbHeight() * 0.5, 0);
        stopPilot();
        discard();
        if (level() instanceof ServerLevel server) BlastExecutor.detonate(server, center, axis, warhead().profile(), null);
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
            ItemStack stack = new ItemStack(KsRegistries.itemFor(warhead()));
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
    @Override protected void readAdditionalSaveData(ValueInput in) { simInitialised = false; setWarhead(Warhead.byOrdinal(in.getIntOr("Warhead", 0))); }
    @Override protected void addAdditionalSaveData(ValueOutput out) { out.putInt("Warhead", warhead().ordinal()); }

    private static Vec3d toSim(Vec3 v) { return new Vec3d(v.x, v.y, v.z); }
}
