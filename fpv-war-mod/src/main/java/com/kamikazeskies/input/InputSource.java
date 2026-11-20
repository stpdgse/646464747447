package com.kamikazeskies.input;

import com.kamikazeskies.sim.FlightInput;

/** A device that turns raw hardware state into normalized {@link FlightInput}. Keyboard, gamepad and touch all implement this. */
public interface InputSource {
    /** @param dt seconds since the previous sample */
    FlightInput sample(double dt);

    /** True when the user is actively touching this device (used to pick the active source). */
    boolean isActive();
}
