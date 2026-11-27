package com.openfpv.item;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.HashSet;
import java.util.Set;
import org.junit.Test;

public class ModuleTypeTest {
    @Test
    public void everyModuleHasItsOwnBit() {
        int all = 0;
        for (ModuleType t : ModuleType.VALUES) {
            assertEquals(0, all & t.bit());
            all |= t.bit();
        }
        assertEquals((1 << ModuleType.VALUES.length) - 1, all);
    }

    @Test
    public void maskQueries() {
        int mask = ModuleType.ZOOM.bit() | ModuleType.DEPTH.bit();
        assertTrue(ModuleType.has(mask, ModuleType.ZOOM));
        assertTrue(ModuleType.has(mask, ModuleType.DEPTH));
        assertFalse(ModuleType.has(mask, ModuleType.THERMAL));
        assertFalse(ModuleType.has(0, ModuleType.GIMBAL));
    }

    @Test
    public void namesAreUnique() {
        Set<String> items = new HashSet<>(), lua = new HashSet<>();
        for (ModuleType t : ModuleType.VALUES) {
            assertTrue("duplicate item name " + t.itemName, items.add(t.itemName));
            assertTrue("duplicate lua name " + t.luaName, lua.add(t.luaName));
            assertTrue(t.itemName.endsWith("_module"));
        }
        assertEquals(5, ModuleType.VALUES.length);
    }
}
