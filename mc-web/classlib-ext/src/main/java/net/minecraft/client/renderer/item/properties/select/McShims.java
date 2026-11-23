package net.minecraft.client.renderer.item.properties.select;

import com.mojang.serialization.DataResult;
import java.util.Optional;

/**
 * Replacements for the one item property that needs ICU's calendar (local_time). ICU's time zone data is not
 * available in the browser build, so such item models report a decode error instead of crashing the loader.
 * Lives in the game's package because the property's data record is package-private.
 */
public final class McShims {
    private McShims() {
    }

    public static DataResult<LocalTime> localTimeUnsupported(Object data) {
        return DataResult.error(() -> "The local_time item property is not available in the browser port");
    }

    public static LocalTime localTimeUnsupported(String format, String locale, Optional<?> zone) {
        throw new IllegalStateException("The local_time item property is not available in the browser port");
    }
}
