package com.openfpv.item;

/** The camera modules that can be installed in a Camera Upgrade's module bay. */
public enum ModuleType {
    NIGHT_VISION("night_vision_module", "nightvision"),
    ZOOM("zoom_module", "zoom"),
    GIMBAL("gimbal_module", "gimbal"),
    THERMAL("thermal_module", "thermal"),
    DEPTH("depth_module", "depth");

    public static final ModuleType[] VALUES = values();

    /** Registry path of the module item. */
    public final String itemName;
    /** Name reported to Lua by camera.getModules(). */
    public final String luaName;

    ModuleType(String itemName, String luaName) {
        this.itemName = itemName;
        this.luaName = luaName;
    }

    public int bit() {
        return 1 << ordinal();
    }

    public static boolean has(int mask, ModuleType type) {
        return (mask & type.bit()) != 0;
    }
}
