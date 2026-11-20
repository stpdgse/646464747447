package com.kamikazeskies.core;

import com.kamikazeskies.drone.TrainingRange;
import com.mojang.brigadier.CommandDispatcher;

import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.item.ItemStack;

/** {@code /kamikaze ...} developer and practice commands (operators only). */
public final class KsCommands {
    private KsCommands() {}

    public static void register(CommandDispatcher<CommandSourceStack> dispatcher) {
        dispatcher.register(Commands.literal("kamikaze")
                .requires(Commands.hasPermission(Commands.LEVEL_GAMEMASTERS))
                .then(Commands.literal("training").executes(ctx -> {
                    ServerPlayer player = ctx.getSource().getPlayerOrException();
                    Direction facing = player.getDirection();
                    BlockPos origin = player.blockPosition();
                    int placed = TrainingRange.build(player.level() instanceof net.minecraft.server.level.ServerLevel l ? l : ctx.getSource().getLevel(), origin, facing);
                    ItemStack drone = new ItemStack(KsRegistries.DRONE_ITEM.get());
                    if (!player.getInventory().add(drone)) player.spawnAtLocation(ctx.getSource().getLevel(), drone);
                    ctx.getSource().sendSuccess(() -> Component.translatable("message.kamikaze_skies.training_built", placed), false);
                    return placed;
                })));
    }
}
