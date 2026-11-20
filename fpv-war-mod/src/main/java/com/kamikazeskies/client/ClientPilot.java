package com.kamikazeskies.client;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.input.GamepadSticks;
import com.kamikazeskies.input.KeyboardSticks;
import com.kamikazeskies.input.PilotSwitches;
import com.kamikazeskies.net.ExitPilotPayload;
import com.kamikazeskies.net.PilotInputPayload;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.player.ClientInput;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.world.entity.Entity;
import net.neoforged.neoforge.client.network.ClientPacketDistributor;

/**
 * Client-side pilot session. Starts when the server marks this player as a drone's pilot (synced entity data),
 * then: puts the camera on the drone, freezes the operator's own movement, samples the active input device each
 * tick and sends it to the server, and cleans everything up when the link ends.
 */
public final class ClientPilot {
    private static final double TICK_DT = 0.05;

    private static @Nullable DroneEntity drone;
    private static PilotSwitches switches = new PilotSwitches();
    private static KeyboardSticks keyboard = new KeyboardSticks(switches);
    private static GamepadSticks gamepad = new GamepadSticks(switches);
    private static boolean usingGamepad;
    private static FlightInput lastInput = FlightInput.DISARMED;
    private static int armedTicks;
    private static int beepClock;

    private static @Nullable ClientInput savedInput;
    private static float savedYaw, savedPitch;
    private static @Nullable CameraType savedCamera;

    private ClientPilot() {}

    public static boolean isPiloting() { return drone != null; }
    public static @Nullable DroneEntity drone() { return drone; }
    public static FlightInput lastInput() { return lastInput; }
    public static PilotSwitches switches() { return switches; }
    public static boolean usingGamepad() { return usingGamepad; }
    /** Seconds spent armed in this session (the OSD flight timer). */
    public static int flightSeconds() { return armedTicks / 20; }

    /** Called every client tick (post). */
    public static void tick() {
        Minecraft mc = Minecraft.getInstance();
        LocalPlayer player = mc.player;
        ClientLevel level = mc.level;
        if (player == null || level == null) {
            if (drone != null) end(mc, null);
            return;
        }

        DroneEntity found = findPilotedDrone(level, player.getId());
        if (found != null && drone == null) begin(mc, player, found);
        else if (found == null && drone != null) end(mc, player);
        else if (found != null && found != drone) drone = found;
        if (drone == null) return;

        if (drone.isArmed()) armedTicks++;
        warningBeeps(mc, drone);

        // keep the operator standing still and looking where they were
        player.setYRot(savedYaw);
        player.setXRot(savedPitch);

        // --- sample devices ---
        SdlGamepad.Sample pad = SdlGamepad.poll();
        keyboard.setKeys(PilotKeys.held());
        boolean toggleArm = false, cycleAssist = false, exit = false;
        while (PilotKeys.ARM.consumeClick()) toggleArm = true;
        while (PilotKeys.ASSIST.consumeClick()) cycleAssist = true;
        while (PilotKeys.EXIT.consumeClick()) exit = true;
        if (pad != null) {
            gamepad.setAxes(pad.axes());
            toggleArm |= pad.buttons().arm();
            cycleAssist |= pad.buttons().assist();
            exit |= pad.buttons().exit();
            if (gamepad.isActive()) usingGamepad = true;
        }
        if (keyboard.isActive()) usingGamepad = false;
        if (pad == null) usingGamepad = false;

        FlightInput kb = keyboard.sample(TICK_DT);
        FlightInput gp = gamepad.sample(TICK_DT);
        double throttleNow = usingGamepad ? gp.throttle() : kb.throttle();
        if (toggleArm) switches.toggleArm(throttleNow);
        if (cycleAssist) switches.cycleAssist();

        if (exit) {
            ClientPacketDistributor.sendToServer(ExitPilotPayload.INSTANCE);
            return;
        }
        // re-sample so switch changes made this tick are included immediately
        FlightInput in = usingGamepad ? gamepad.sample(0) : keyboard.sample(0);
        lastInput = in;
        ClientPacketDistributor.sendToServer(PilotInputPayload.of(in));
    }

    /** Radio-style warning tones: slow beep for a low pack, fast double beep when critical, short chirps on weak signal. */
    private static void warningBeeps(Minecraft mc, DroneEntity d) {
        beepClock++;
        boolean critical = d.batterySoc() < 0.05f, low = d.batteryLow();
        boolean lost = d.rssi() < 0.08f;
        int period = critical ? 6 : low ? 20 : lost ? 12 : 0;
        if (period == 0 || beepClock % period != 0) return;
        float pitch = critical ? 1.9f : low ? 1.4f : 0.8f;
        mc.getSoundManager().play(SimpleSoundInstance.forUI(SoundEvents.NOTE_BLOCK_PLING.value(), pitch, 0.7f));
    }

    private static @Nullable DroneEntity findPilotedDrone(ClientLevel level, int playerId) {
        for (Entity e : level.entitiesForRendering()) {
            if (e instanceof DroneEntity d && d.pilotEntityId() == playerId) return d;
        }
        return null;
    }

    private static void begin(Minecraft mc, LocalPlayer player, DroneEntity d) {
        drone = d;
        switches = new PilotSwitches();
        keyboard = new KeyboardSticks(switches);
        gamepad = new GamepadSticks(switches);
        usingGamepad = false;
        lastInput = FlightInput.DISARMED;
        armedTicks = 0;
        beepClock = 0;

        savedInput = player.input;
        player.input = new ClientInput();           // inert: no walking, jumping or sneaking while flying
        savedYaw = player.getYRot();
        savedPitch = player.getXRot();
        savedCamera = mc.options.getCameraType();
        mc.options.setCameraType(CameraType.FIRST_PERSON);
        mc.setCameraEntity(d);
    }

    private static void end(Minecraft mc, @Nullable LocalPlayer player) {
        drone = null;
        if (player != null) {
            if (savedInput != null) player.input = savedInput;
            mc.setCameraEntity(player);
        } else {
            mc.setCameraEntity(null);
        }
        if (savedCamera != null) mc.options.setCameraType(savedCamera);
        savedInput = null;
        savedCamera = null;
        lastInput = FlightInput.DISARMED;
    }
}
