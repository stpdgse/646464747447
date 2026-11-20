package com.kamikazeskies.drone;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.destruction.core.BlastProfile;

/** The payload a drone carries. {@link #NONE} drones only crash. */
public enum Warhead {
    NONE(null),
    IMPACT(BlastProfile.IMPACT),
    SHAPED(BlastProfile.SHAPED);

    private final @Nullable BlastProfile profile;

    Warhead(@Nullable BlastProfile profile) { this.profile = profile; }

    public @Nullable BlastProfile profile() { return profile; }
    public boolean armed() { return profile != null; }

    public static Warhead byOrdinal(int i) { return i >= 0 && i < values().length ? values()[i] : NONE; }
}
