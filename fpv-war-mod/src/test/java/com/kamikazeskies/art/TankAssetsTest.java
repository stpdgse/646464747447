package com.kamikazeskies.art;

import static org.junit.jupiter.api.Assertions.*;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Set;

import org.junit.jupiter.api.Test;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.kamikazeskies.drone.TankAnim;

class TankAssetsTest {
    private static InputStream res(String path) { return TankAssetsTest.class.getResourceAsStream("/assets/kamikaze_skies/" + path); }
    private static JsonObject load(String path) throws Exception {
        try (InputStream in = res(path)) { assertNotNull(in, "missing " + path); return JsonParser.parseString(new String(in.readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject(); }
    }

    @Test void modelAnimationsAndTexturesAgree() throws Exception {
        JsonObject geo = load("geckolib/models/entity/tank.geo.json").getAsJsonArray("minecraft:geometry").get(0).getAsJsonObject();
        assertEquals("geometry.kamikaze_skies.tank", geo.getAsJsonObject("description").get("identifier").getAsString());
        Set<String> bones = new HashSet<>();
        geo.getAsJsonArray("bones").forEach(b -> bones.add(b.getAsJsonObject().get("name").getAsString()));
        assertTrue(bones.containsAll(Set.of("chassis", "turret", "gun", "barrel", "mg", "antenna", "wheel_l0", "wheel_r5")));
        JsonObject anims = load("geckolib/animations/entity/tank.animation.json").getAsJsonObject("animations");
        for (var e : anims.entrySet()) for (String b : e.getValue().getAsJsonObject().getAsJsonObject("bones").keySet()) assertTrue(bones.contains(b), e.getKey() + " animates unknown bone " + b);
        for (String n : new String[]{"idle", "drive", "turret_scan", "fire", "destroyed"}) assertTrue(anims.has(TankAnim.PREFIX + n), "missing animation " + n);
        assertEquals("hold_on_last_frame", anims.getAsJsonObject(TankAnim.PREFIX + "destroyed").get("loop").getAsString());
        assertFalse(anims.getAsJsonObject(TankAnim.PREFIX + "fire").get("loop").getAsBoolean());
        assertNotNull(res("textures/entity/tank.png")); assertNotNull(res("textures/entity/tank_wreck.png"));
    }

    @Test void hullAnimationFollowsState() {
        assertEquals("idle", TankAnim.hull(false, 0));
        assertEquals("drive", TankAnim.hull(false, 0.1));
        assertEquals("destroyed", TankAnim.hull(true, 0.1));
    }
}
