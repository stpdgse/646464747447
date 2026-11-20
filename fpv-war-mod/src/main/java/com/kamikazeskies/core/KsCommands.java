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
                .then(Commands.literal("warzone").executes(ctx -> {
                    ServerPlayer p = ctx.getSource().getPlayerOrException();
                    net.minecraft.server.level.ServerLevel wz = ctx.getSource().getServer().getLevel(com.kamikazeskies.mission.MissionManager.WARZONE);
                    if (wz == null) { ctx.getSource().sendFailure(Component.literal("The warzone dimension is not loaded")); return 0; }
                    int y = wz.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, 0, 0);
                    p.teleportTo(wz, 0.5, y, 0.5, java.util.Set.of(), p.getYRot(), 0f, true);
                    return 1;
                }))
                .then(Commands.literal("mission")
                        .then(Commands.literal("list").executes(ctx -> {
                            for (var m : com.kamikazeskies.mission.MissionRegistry.all().values()) ctx.getSource().sendSuccess(() -> Component.literal(m.id() + ": " + m.title()), false);
                            return com.kamikazeskies.mission.MissionRegistry.all().size();
                        }))
                        .then(Commands.literal("start").then(Commands.argument("id", com.mojang.brigadier.arguments.StringArgumentType.word())
                                .suggests((c, b) -> net.minecraft.commands.SharedSuggestionProvider.suggest(com.kamikazeskies.mission.MissionRegistry.all().keySet(), b))
                                .executes(ctx -> {
                                    var def = com.kamikazeskies.mission.MissionRegistry.all().get(com.mojang.brigadier.arguments.StringArgumentType.getString(ctx, "id"));
                                    if (def == null) { ctx.getSource().sendFailure(Component.literal("Unknown mission")); return 0; }
                                    if (!com.kamikazeskies.mission.MissionManager.start(ctx.getSource().getPlayerOrException(), def)) { ctx.getSource().sendFailure(Component.literal("You already have a mission running")); return 0; }
                                    return 1;
                                })))
                        .then(Commands.literal("status").executes(ctx -> {
                            var st = com.kamikazeskies.mission.MissionManager.stateOf(ctx.getSource().getPlayerOrException());
                            if (st == null) { ctx.getSource().sendSuccess(() -> Component.literal("No mission running"), false); return 0; }
                            for (int i = 0; i < st.def().objectives().size(); i++) { int k = i; ctx.getSource().sendSuccess(() -> Component.literal(st.def().objectives().get(k).text() + ": " + st.progress(k) + "/" + st.def().objectives().get(k).count()), false); }
                            return 1;
                        }))
                        .then(Commands.literal("abort").executes(ctx -> { com.kamikazeskies.mission.MissionManager.abort(ctx.getSource().getPlayerOrException()); return 1; })))
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
