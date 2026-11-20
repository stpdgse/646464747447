package com.kamikazeskies.mission;

import java.util.ArrayList;
import java.util.List;

/** Progress of one running mission. Pure Java, advanced by the server glue once per tick and on kills. */
public final class MissionState {
    public enum Status { ACTIVE, COMPLETE, FAILED }

    private final MissionDef def;
    private final int[] progress;
    private Status status = Status.ACTIVE;
    private String failReason = "";
    private int elapsedTicks;
    private final List<MissionDef.Bonus> earnedBonus = new ArrayList<>();

    public MissionState(MissionDef def) { this.def = def; this.progress = new int[def.objectives().size()]; }

    public MissionDef def() { return def; }
    public Status status() { return status; }
    public String failReason() { return failReason; }
    public int elapsedTicks() { return elapsedTicks; }
    public int progress(int objective) { return progress[objective]; }
    public List<MissionDef.Bonus> earnedBonus() { return earnedBonus; }
    public boolean objectiveDone(int i) { return progress[i] >= def.objectives().get(i).count(); }
    public int remainingTicks() { return def.timeLimitSeconds() <= 0 ? Integer.MAX_VALUE : Math.max(0, def.timeLimitSeconds() * 20 - elapsedTicks); }

    /** @return true if this kill advanced an objective */
    public boolean onKill(String entity) {
        if (status != Status.ACTIVE) return false;
        boolean advanced = false;
        for (int i = 0; i < progress.length; i++) {
            if (def.objectives().get(i).entity().equals(entity) && !objectiveDone(i)) { progress[i]++; advanced = true; }
        }
        if (advanced) checkComplete();
        return advanced;
    }

    public void onOperatorDied() { fail("The operator was killed"); }
    public void abort() { fail("Aborted"); }

    public void tick() {
        if (status != Status.ACTIVE) return;
        elapsedTicks++;
        if (def.timeLimitSeconds() > 0 && elapsedTicks > def.timeLimitSeconds() * 20) fail("Out of time");
    }

    private void checkComplete() {
        for (int i = 0; i < progress.length; i++) if (!objectiveDone(i)) return;
        status = Status.COMPLETE;
        for (MissionDef.Bonus b : def.bonus()) if (b.seconds() <= 0 || elapsedTicks <= b.seconds() * 20) earnedBonus.add(b);
    }

    private void fail(String reason) { if (status == Status.ACTIVE) { status = Status.FAILED; failReason = reason; } }
}
