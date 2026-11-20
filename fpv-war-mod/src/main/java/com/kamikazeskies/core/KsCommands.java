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

    /** {@code kind} is a stance (stand, aim, reload, crouch, panic) or a role (conscript, rifleman, veteran). */
    private static int spawnSoldier(CommandSourceStack src, String kind) {
        net.minecraft.server.level.ServerLevel level = src.getLevel();
        com.kamikazeskies.drone.SoldierEntity s = KsRegistries.SOLDIER.get().create(level, net.minecraft.world.entity.EntitySpawnReason.COMMAND);
        if (s == null) return 0;
        String k = kind.toUpperCase(java.util.Locale.ROOT);
        try { s.setStance(com.kamikazeskies.drone.SoldierAnim.Stance.valueOf(k)); }
        catch (IllegalArgumentException notStance) {
            try { s.setRole(com.kamikazeskies.ai.Role.valueOf(k)); }
            catch (IllegalArgumentException notRole) { src.sendFailure(Component.literal("Unknown stance or role: " + kind)); return 0; }
        }
        s.snapTo(src.getPosition().x, src.getPosition().y, src.getPosition().z, src.getRotation().y + 180f, 0f);
        level.addFreshEntity(s);
        src.sendSuccess(() -> Component.literal("Spawned a " + s.role().name().toLowerCase() + " (" + kind.toLowerCase() + ")"), false);
        return 1;
    }

    public static void register(CommandDispatcher<CommandSourceStack> dispatcher) {
        dispatcher.register(Commands.literal("kamikaze")
                .requires(Commands.hasPermission(Commands.LEVEL_GAMEMASTERS))
                .then(Commands.literal("soldier")
                        .executes(ctx -> spawnSoldier(ctx.getSource(), "stand"))
                        .then(Commands.argument("kind", com.mojang.brigadier.arguments.StringArgumentType.word())
                                .suggests((c, bld) -> net.minecraft.commands.SharedSuggestionProvider.suggest(java.util.stream.Stream.concat(
                                        java.util.Arrays.stream(com.kamikazeskies.drone.SoldierAnim.Stance.values()).map(x -> x.name().toLowerCase()),
                                        java.util.Arrays.stream(com.kamikazeskies.ai.Role.values()).map(x -> x.name().toLowerCase())), bld))
                                .executes(ctx -> spawnSoldier(ctx.getSource(), com.mojang.brigadier.arguments.StringArgumentType.getString(ctx, "kind")))))
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
