package com.kamikazeskies.mission;

import java.util.ArrayList;
import java.util.List;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

/** A mission, loaded from JSON (see {@code data/kamikaze_skies/missions}). Pure Java. */
public record MissionDef(String id, String title, String briefing, int timeLimitSeconds, List<Spawn> spawns, List<Objective> objectives,
                         List<Bonus> bonus, List<Reward> rewards) {
    /** {@code entity} is "soldier" or "tank"; the group is placed {@code dx,dz} metres from the mission origin, scattered within {@code radius}. */
    public record Spawn(String entity, String role, int count, int dx, int dz, int radius) {}
    /** Kill {@code count} of {@code entity}. */
    public record Objective(String entity, int count, String text) {}
    /** Optional goal: finish within {@code seconds}. */
    public record Bonus(int seconds, String text) {}
    public record Reward(String item, int count) {}

    public static MissionDef parse(String json) {
        JsonObject o = JsonParser.parseString(json).getAsJsonObject();
        List<Spawn> spawns = new ArrayList<>();
        for (JsonElement e : arr(o, "spawns")) { JsonObject s = e.getAsJsonObject(); spawns.add(new Spawn(s.get("entity").getAsString(), str(s, "role", ""), num(s, "count", 1), num(s, "dx", 0), num(s, "dz", 0), num(s, "radius", 3))); }
        List<Objective> objectives = new ArrayList<>();
        for (JsonElement e : arr(o, "objectives")) { JsonObject s = e.getAsJsonObject(); objectives.add(new Objective(s.get("entity").getAsString(), num(s, "count", 1), str(s, "text", ""))); }
        List<Bonus> bonus = new ArrayList<>();
        for (JsonElement e : arr(o, "bonus")) { JsonObject s = e.getAsJsonObject(); bonus.add(new Bonus(num(s, "seconds", 0), str(s, "text", ""))); }
        List<Reward> rewards = new ArrayList<>();
        for (JsonElement e : arr(o, "rewards")) { JsonObject s = e.getAsJsonObject(); rewards.add(new Reward(s.get("item").getAsString(), num(s, "count", 1))); }
        if (objectives.isEmpty()) throw new IllegalArgumentException("mission " + o.get("id") + " has no objectives");
        return new MissionDef(o.get("id").getAsString(), o.get("title").getAsString(), str(o, "briefing", ""), num(o, "time_limit_seconds", 0), spawns, objectives, bonus, rewards);
    }

    private static JsonArray arr(JsonObject o, String k) { return o.has(k) ? o.getAsJsonArray(k) : new JsonArray(); }
    private static String str(JsonObject o, String k, String d) { return o.has(k) ? o.get(k).getAsString() : d; }
    private static int num(JsonObject o, String k, int d) { return o.has(k) ? o.get(k).getAsInt() : d; }
}
