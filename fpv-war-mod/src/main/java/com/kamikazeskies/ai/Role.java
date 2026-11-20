package com.kamikazeskies.ai;

/** Soldier temperament: how readily he panics and how well he shoots. Values are deliberately spread so a squad never reacts as one. */
public enum Role {
    CONSCRIPT(0.25, 0.25, 40), RIFLEMAN(0.55, 0.45, 45), VETERAN(0.85, 0.7, 15);

    public final double courage, skill;
    public final int spawnWeight;
    Role(double courage, double skill, int spawnWeight) { this.courage = courage; this.skill = skill; this.spawnWeight = spawnWeight; }

    /** Weighted pick; {@code r} in [0,1). */
    public static Role pick(double r) {
        int total = 0; for (Role x : values()) total += x.spawnWeight;
        double t = r * total;
        for (Role x : values()) { t -= x.spawnWeight; if (t < 0) return x; }
        return RIFLEMAN;
    }
}
