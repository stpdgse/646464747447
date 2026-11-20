package com.kamikazeskies.client;


import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.input.KeyboardSticks;
import com.mojang.blaze3d.platform.InputConstants;

import net.minecraft.client.KeyMapping;
import net.minecraft.resources.Identifier;
import net.neoforged.neoforge.client.event.RegisterKeyMappingsEvent;
import net.neoforged.neoforge.client.settings.IKeyConflictContext;

/**
 * Pilot key bindings. They live in their own conflict context that is only active while piloting, so the
 * default WASD/Space/Shift keys can be reused without clashing with vanilla movement.
 */
public final class PilotKeys {
    /** Conflicts only with other pilot keys; active only while the goggles are on. */
    public enum Context implements IKeyConflictContext {
        PILOTING;

        @Override public boolean isActive() { return ClientPilot.isPiloting(); }
        @Override public boolean conflicts(IKeyConflictContext other) { return other == this; }
    }

    public static final KeyMapping.Category CATEGORY =
            new KeyMapping.Category(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "flight"));

    public static final KeyMapping PITCH_FORWARD = key("pitch_forward", InputConstants.KEY_W);
    public static final KeyMapping PITCH_BACK = key("pitch_back", InputConstants.KEY_S);
    public static final KeyMapping ROLL_LEFT = key("roll_left", InputConstants.KEY_A);
    public static final KeyMapping ROLL_RIGHT = key("roll_right", InputConstants.KEY_D);
    public static final KeyMapping YAW_LEFT = key("yaw_left", InputConstants.KEY_Q);
    public static final KeyMapping YAW_RIGHT = key("yaw_right", InputConstants.KEY_E);
    public static final KeyMapping THROTTLE_UP = key("throttle_up", InputConstants.KEY_SPACE);
    public static final KeyMapping THROTTLE_DOWN = key("throttle_down", InputConstants.KEY_LSHIFT);
    public static final KeyMapping ARM = key("arm", InputConstants.KEY_R);
    public static final KeyMapping ASSIST = key("assist_mode", InputConstants.KEY_F);
    public static final KeyMapping EXIT = key("exit_goggles", InputConstants.KEY_G);

    private PilotKeys() {}

    private static KeyMapping key(String name, int glfwKey) {
        return new KeyMapping("key." + KamikazeSkies.MODID + "." + name, Context.PILOTING, InputConstants.Type.KEYBOARD, glfwKey, CATEGORY);
    }

    public static void register(RegisterKeyMappingsEvent event) {
        event.registerCategory(CATEGORY);
        for (KeyMapping k : new KeyMapping[] {PITCH_FORWARD, PITCH_BACK, ROLL_LEFT, ROLL_RIGHT, YAW_LEFT, YAW_RIGHT,
                THROTTLE_UP, THROTTLE_DOWN, ARM, ASSIST, EXIT}) {
            event.register(k);
        }
    }

    /** Current hold-state of the flight keys. */
    public static KeyboardSticks.Keys held() {
        return new KeyboardSticks.Keys(PITCH_FORWARD.isDown(), PITCH_BACK.isDown(), ROLL_LEFT.isDown(), ROLL_RIGHT.isDown(),
                YAW_LEFT.isDown(), YAW_RIGHT.isDown(), THROTTLE_UP.isDown(), THROTTLE_DOWN.isDown());
    }
}
