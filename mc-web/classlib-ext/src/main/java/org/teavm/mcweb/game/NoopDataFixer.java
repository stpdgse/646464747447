package org.teavm.mcweb.game;

import com.mojang.datafixers.DSL;
import com.mojang.datafixers.DataFixer;
import com.mojang.datafixers.schemas.Schema;
import com.mojang.serialization.Dynamic;

/**
 * A data fixer that changes nothing. Worlds created in the browser are always at the current data
 * version, so the world-upgrade machinery (which drags in heavy generic-type reflection) is skipped.
 */
public final class NoopDataFixer implements DataFixer {
    public static final NoopDataFixer INSTANCE = new NoopDataFixer();

    private NoopDataFixer() {
    }

    @Override
    public <T> Dynamic<T> update(DSL.TypeReference type, Dynamic<T> input, int version, int newVersion) {
        return input;
    }

    @Override
    public Schema getSchema(int key) {
        org.teavm.mcweb.host.WebHost.logStack("[gap] DataFixer.getSchema(" + key + ")");
        throw new UnsupportedOperationException("data fixer schemas are not available in the browser port");
    }
}
