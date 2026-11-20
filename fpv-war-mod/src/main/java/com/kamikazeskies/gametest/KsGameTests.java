package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.level.block.Blocks;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;

/**
 * GameTest functions. Each function is registered in the TEST_FUNCTION registry and referenced from a
 * JSON file in {@code data/kamikaze_skies/test_instance/}. Run all with {@code ./gradlew runGameTestServer}.
 */
public final class KsGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);

    /** Phase 0 smoke test: the world is live, blocks can be placed and read back. */
    public static final DeferredHolder<Consumer<GameTestHelper>, Consumer<GameTestHelper>> SMOKE =
            FUNCTIONS.register("smoke", () -> KsGameTests::smoke);

    private KsGameTests() {}

    public static void register(IEventBus modEventBus) {
        FUNCTIONS.register(modEventBus);
    }

    private static void smoke(GameTestHelper helper) {
        BlockPos pos = new BlockPos(1, 1, 1);
        helper.setBlock(pos, Blocks.STONE);
        helper.assertBlock(pos, b -> b == Blocks.STONE, b -> Component.literal("expected stone, found " + b));
        helper.setBlock(pos, Blocks.AIR);
        helper.assertBlock(pos, b -> b == Blocks.AIR, b -> Component.literal("expected air, found " + b));
        helper.succeed();
    }
}
