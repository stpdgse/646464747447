package com.openfpv.oc;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Server-side lookup from drone entity id to its camera component. */
public final class CameraRegistry {
    private static final Map<Integer, CameraEnvironment> CAMERAS = new ConcurrentHashMap<>();

    private CameraRegistry() {}

    public static void register(int entityId, CameraEnvironment env) {
        CAMERAS.put(entityId, env);
    }

    public static void unregister(int entityId, CameraEnvironment env) {
        CAMERAS.remove(entityId, env);
    }

    public static CameraEnvironment get(int entityId) {
        return CAMERAS.get(entityId);
    }

    public static void clear() {
        CAMERAS.clear();
    }
}
