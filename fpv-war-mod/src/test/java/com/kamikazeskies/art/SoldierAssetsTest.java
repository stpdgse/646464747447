package com.kamikazeskies.art;

import static org.junit.jupiter.api.Assertions.*;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Set;

import org.junit.jupiter.api.Test;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.kamikazeskies.drone.SoldierAnim;
import com.kamikazeskies.drone.SoldierAnim.Stance;

/** The rifleman's model, animations and entity logic must agree with each other. */
class SoldierAssetsTest {
    private static JsonObject load(String path) throws Exception {
        try (InputStream in = SoldierAssetsTest.class.getResourceAsStream("/assets/kamikaze_skies/" + path)) {
            assertNotNull(in, "missing resource " + path);
            return JsonParser.parseString(new String(in.readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject();
        }
    }

    private static Set<String> geoBones() throws Exception {
        Set<String> bones = new HashSet<>();
        for (JsonElement b : load("geckolib/models/entity/rifleman.geo.json").getAsJsonArray("minecraft:geometry").get(0).getAsJsonObject().getAsJsonArray("bones"))
            bones.add(b.getAsJsonObject().get("name").getAsString());
        return bones;
    }

    @Test void geometryHasTheRightIdentifierAndBones() throws Exception {
        JsonObject geo = load("geckolib/models/entity/rifleman.geo.json").getAsJsonArray("minecraft:geometry").get(0).getAsJsonObject();
        assertEquals("geometry.kamikaze_skies.rifleman", geo.getAsJsonObject("description").get("identifier").getAsString());
        assertTrue(geoBones().containsAll(Set.of("root", "torso", "head", "arm_r", "arm_r_lower", "arm_l", "arm_l_lower", "leg_r", "leg_l", "rifle", "mag")));
    }

    @Test void everyAnimatedBoneExistsInTheModel() throws Exception {
        Set<String> bones = geoBones();
        JsonObject anims = load("geckolib/animations/entity/rifleman.animation.json").getAsJsonObject("animations");
        for (var e : anims.entrySet())
            for (String b : e.getValue().getAsJsonObject().getAsJsonObject("bones").keySet())
                assertTrue(bones.contains(b), e.getKey() + " animates unknown bone " + b);
    }

    @Test void everyAnimationTheEntityPlaysExists() throws Exception {
        JsonObject anims = load("geckolib/animations/entity/rifleman.animation.json").getAsJsonObject("animations");
        Set<String> needed = new HashSet<>(Set.of("fire", "flinch", "reload", "idle", "walk", "run", "aim", "aim_idle", "crouch", "crouch_idle", "panic", "flee", "death_a", "death_b"));
        for (Stance st : Stance.values()) for (boolean dead : new boolean[]{false, true}) for (double speed : new double[]{0, 0.05, 0.3})
            needed.add(SoldierAnim.steady(dead, dead && speed > 0.1, st, speed));
        for (String n : needed) assertTrue(anims.has(SoldierAnim.PREFIX + n), "animation missing: " + n);
    }

    @Test void deathAnimationsHoldTheirLastFrameAndLoopsLoop() throws Exception {
        JsonObject anims = load("geckolib/animations/entity/rifleman.animation.json").getAsJsonObject("animations");
        for (String n : new String[]{"death_a", "death_b", "aim", "crouch"}) assertEquals("hold_on_last_frame", anims.getAsJsonObject(SoldierAnim.PREFIX + n).get("loop").getAsString(), n);
        for (String n : new String[]{"idle", "walk", "run", "aim_idle", "crouch_idle", "panic", "flee"}) assertTrue(anims.getAsJsonObject(SoldierAnim.PREFIX + n).get("loop").getAsBoolean(), n + " must loop");
        for (String n : new String[]{"fire", "flinch", "reload"}) assertFalse(anims.getAsJsonObject(SoldierAnim.PREFIX + n).get("loop").getAsBoolean(), n + " must be one-shot");
    }

    @Test void loopingAnimationsCloseSeamlessly() throws Exception {
        JsonObject anims = load("geckolib/animations/entity/rifleman.animation.json").getAsJsonObject("animations");
        for (String n : new String[]{"idle", "walk", "run", "aim_idle", "crouch_idle", "panic", "flee"}) {
            for (var b : anims.getAsJsonObject(SoldierAnim.PREFIX + n).getAsJsonObject("bones").entrySet())
                for (var ch : b.getValue().getAsJsonObject().entrySet()) {
                    JsonObject keys = ch.getValue().getAsJsonObject();
                    if (keys.size() < 2) continue;
                    var first = keys.getAsJsonArray(keys.keySet().iterator().next());
                    JsonElement last = null; for (String k : keys.keySet()) last = keys.get(k);
                    for (int i = 0; i < 3; i++)
                        assertEquals(first.get(i).getAsDouble(), last.getAsJsonArray().get(i).getAsDouble(), 0.01, n + "/" + b.getKey() + "/" + ch.getKey() + " does not loop cleanly");
                }
        }
    }

    @Test void steadyAnimationFollowsStateAndSpeed() {
        assertEquals("idle", SoldierAnim.steady(false, false, Stance.STAND, 0));
        assertEquals("walk", SoldierAnim.steady(false, false, Stance.STAND, 0.1));
        assertEquals("run", SoldierAnim.steady(false, false, Stance.STAND, 0.3));
        assertEquals("aim", SoldierAnim.steady(false, false, Stance.AIM, 0));
        assertEquals("panic", SoldierAnim.steady(false, false, Stance.PANIC, 0));
        assertEquals("flee", SoldierAnim.steady(false, false, Stance.PANIC, 0.3));
        assertEquals("death_a", SoldierAnim.steady(true, false, Stance.AIM, 0.3));
        assertEquals("death_b", SoldierAnim.steady(true, true, Stance.STAND, 0));
    }
}
