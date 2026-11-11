import java.io.PrintWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.IdentityHashMap;
import java.util.Map;

import com.mojang.brigadier.tree.CommandNode;
import com.mojang.brigadier.tree.RootCommandNode;
import io.netty.buffer.Unpooled;
import net.minecraft.SharedConstants;
import net.minecraft.commands.CommandBuildContext;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.protocol.game.ClientboundCommandsPacket;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.server.permissions.PermissionLevel;

/**
 * Dumps the command tree the dedicated server sends to a player at each permission level: the body
 * of the {@code commands} packet, byte for byte, built exactly as {@code Commands.sendCommands}
 * does it (through its own private helpers).
 *
 * One line per level 0 to 4: {@code level <n> <hex>}.
 */
public class CommandDump {
    @SuppressWarnings({"unchecked", "rawtypes"})
    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Bootstrap.validate();
        CommandBuildContext ctx = Commands.createValidationContext(VanillaRegistries.createWorldLookup());
        Commands commands = new Commands(Commands.CommandSelection.DEDICATED, ctx);
        RootCommandNode<CommandSourceStack> root = commands.getDispatcher().getRoot();
        Method fill = Commands.class.getDeclaredMethod("fillUsableCommands", CommandNode.class, CommandNode.class, Object.class, Map.class);
        fill.setAccessible(true);
        Field inspector = Commands.class.getDeclaredField("COMMAND_NODE_INSPECTOR");
        inspector.setAccessible(true);
        try (PrintWriter w = new PrintWriter(args[0], StandardCharsets.UTF_8)) {
            for (int level = 0; level <= 4; level++) {
                CommandSourceStack source = Commands.createCompilationContext(
                    LevelBasedPermissionSet.forLevel(PermissionLevel.byId(level)));
                Map map = new IdentityHashMap();
                RootCommandNode newRoot = new RootCommandNode();
                map.put(root, newRoot);
                fill.invoke(null, root, newRoot, source, map);
                ClientboundCommandsPacket packet = new ClientboundCommandsPacket(newRoot,
                    (ClientboundCommandsPacket.NodeInspector) inspector.get(null));
                FriendlyByteBuf buf = new FriendlyByteBuf(Unpooled.buffer());
                ClientboundCommandsPacket.STREAM_CODEC.encode(buf, packet);
                byte[] bytes = new byte[buf.readableBytes()];
                buf.readBytes(bytes);
                StringBuilder hex = new StringBuilder();
                for (byte b : bytes) hex.append(String.format("%02x", b));
                w.println("level " + level + " " + hex);
            }
        }
    }
}
