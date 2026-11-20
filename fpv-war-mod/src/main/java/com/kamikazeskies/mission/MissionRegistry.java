package com.kamikazeskies.mission;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;

import com.google.gson.JsonElement;
import com.google.gson.JsonParser;

/** Missions shipped in the mod jar ({@code data/kamikaze_skies/missions/index.json} lists their ids). Pure Java. */
public final class MissionRegistry {
    private static Map<String, MissionDef> cache;
    private MissionRegistry() {}

    public static synchronized Map<String, MissionDef> all() {
        if (cache == null) {
            Map<String, MissionDef> m = new LinkedHashMap<>();
            try {
                for (JsonElement id : JsonParser.parseString(read("index.json")).getAsJsonArray()) m.put(id.getAsString(), MissionDef.parse(read(id.getAsString() + ".json")));
            } catch (IOException e) { throw new IllegalStateException("cannot load missions", e); }
            cache = m;
        }
        return cache;
    }

    private static String read(String name) throws IOException {
        try (InputStream in = MissionRegistry.class.getResourceAsStream("/data/kamikaze_skies/missions/" + name)) {
            if (in == null) throw new IOException("missing mission file " + name);
            return new String(in.readAllBytes(), StandardCharsets.UTF_8);
        }
    }
}
