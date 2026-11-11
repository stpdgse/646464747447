import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStackTemplate;

/**
 * Dumps the crafting remainder of every item that leaves something behind when it is used up in
 * crafting or as furnace fuel (a lava bucket leaves a bucket). The data generator's component
 * reports do not include it.
 *
 * One line per such item: {@code name remainder_name remainder_count}.
 */
public class ItemDump {
    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Bootstrap.validate();
        List<String> rows = new ArrayList<>();
        for (Item item : BuiltInRegistries.ITEM) {
            ItemStackTemplate rem = item.getCraftingRemainder();
            if (rem == null) continue;
            rows.add(BuiltInRegistries.ITEM.getKey(item) + " "
                + BuiltInRegistries.ITEM.getKey(rem.typeHolder().value()) + " " + rem.count());
        }
        try (PrintWriter w = new PrintWriter(args[0], StandardCharsets.UTF_8)) {
            w.println("remainders " + rows.size());
            for (String r : rows) w.println(r);
        }
    }
}
