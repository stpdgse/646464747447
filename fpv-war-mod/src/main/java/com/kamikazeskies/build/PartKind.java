package com.kamikazeskies.build;

/** The kinds of craftable drone part. Tiered parts exist in tiers 1-3; warheads and the blackbox have a single tier. */
public enum PartKind {
    FRAME("frame", true), MOTOR("motors", true), PROP("props", true), BATTERY("battery", true), FLIGHT_CONTROLLER("flight_controller", true),
    CAMERA("camera", true), ANTENNA("antenna", true),
    WARHEAD_IMPACT("warhead_impact", false), WARHEAD_SHAPED("warhead_shaped", false), BLACKBOX("blackbox", false);

    public final String id;
    public final boolean tiered;

    PartKind(String id, boolean tiered) { this.id = id; this.tiered = tiered; }

    public static final int MAX_TIER = 3;
}
