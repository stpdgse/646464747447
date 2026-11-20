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

    private static int spawnSoldier(CommandSourceStack src, com.kamikazeskies.drone.SoldierAnim.Stance stance) {
        net.minecraft.server.level.ServerLevel level = src.getLevel();
        com.kamikazeskies.drone.SoldierEntity s = KsRegistries.SOLDIER.get().create(level, net.minecraft.world.entity.EntitySpawnReason.COMMAND);
        if (s == null) return 0;
        s.snapTo(src.getPosition().x, src.getPosition().y, src.getPosition().z, src.getRotation().y + 180f, 0f);
        s.setStance(stance);
        level.addFreshEntity(s);
        src.sendSuccess(() -> Component.literal("Spawned a rifleman (" + stance.name().toLowerCase() + ")"), false);
        return 1;
    }

    public static void register(CommandDispatcher<CommandSourceStack> dispatcher) {
        dispatcher.register(Commands.literal("kamikaze")
                .requires(Commands.hasPermission(Commands.LEVEL_GAMEMASTERS))
                .then(Commands.literal("soldier")
                        .executes(ctx -> spawnSoldier(ctx.getSource(), com.kamikazeskies.drone.SoldierAnim.Stance.STAND))
                        .then(Commands.argument("stance", com.mojang.brigadier.arguments.StringArgumentType.word())
                                .suggests((c, b) -> net.minecraft.commands.SharedSuggestionProvider.suggest(java.util.Arrays.stream(com.kamikazeskies.drone.SoldierAnim.Stance.values()).map(x -> x.name().toLowerCase()), b))
                                .executes(ctx -> spawnSoldier(ctx.getSource(), com.kamikazeskies.drone.SoldierAnim.Stance.valueOf(
                                        com.mojang.brigadier.arguments.StringArgumentType.getString(ctx, "stance").toUpperCase(java.util.Locale.ROOT))))))
                .then(Commands.literal("tank").executes(ctx -> {
                    CommandSourceStack src = ctx.getSource();
                    com.kamikazeskies.drone.TankEntity t = KsRegistries.TANK.get().create(src.getLevel(), net.minecraft.world.entity.EntitySpawnReason.COMMAND);
                    if (t == null) return 0;
                    t.snapTo(src.getPosition().x, src.getPosition().y, src.getPosition().z, src.getRotation().y + 180f, 0f);
                    src.getLevel().addFreshEntity(t);
                    src.sendSuccess(() -> Component.literal("Spawned a tank"), false);
                    return 1;
                }))
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
