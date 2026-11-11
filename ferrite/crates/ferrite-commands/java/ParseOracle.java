import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import com.google.gson.JsonElement;
import com.mojang.brigadier.ParseResults;
import com.mojang.brigadier.context.CommandContextBuilder;
import com.mojang.brigadier.context.ParsedCommandNode;
import com.mojang.brigadier.exceptions.CommandSyntaxException;
import com.mojang.serialization.JsonOps;
import net.minecraft.SharedConstants;
import net.minecraft.commands.CommandBuildContext;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.ComponentSerialization;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.server.permissions.PermissionLevel;

/**
 * Parses command lines with the official dispatcher. Input: one {@code <level> <line>} per line
 * (the line without a leading slash). Output, one line each:
 * <pre>
 *   OK  &lt;nodes&gt; &lt;executable&gt;
 *   ERR &lt;cursor&gt; &lt;message&gt;
 * </pre>
 * where {@code nodes} are the matched node names ({@code /} between nodes, {@code >} where a
 * redirect starts a new context) and {@code message} is {@code lit:text} for Brigadier's own
 * errors or the component as JSON for Minecraft's.
 */
public class ParseOracle {
    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Bootstrap.validate();
        CommandBuildContext ctx = Commands.createValidationContext(VanillaRegistries.createWorldLookup());
        Commands commands = new Commands(Commands.CommandSelection.DEDICATED, ctx);
        List<String> lines = Files.readAllLines(Path.of(args[0]), StandardCharsets.UTF_8);
        try (PrintWriter w = new PrintWriter(args[1], StandardCharsets.UTF_8)) {
            for (String line : lines) {
                int sp = line.indexOf(' ');
                int level = Integer.parseInt(line.substring(0, sp));
                String input = line.substring(sp + 1);
                CommandSourceStack source = Commands.createCompilationContext(
                    LevelBasedPermissionSet.forLevel(PermissionLevel.byId(level)));
                String result;
                try {
                    ParseResults<CommandSourceStack> parse = commands.getDispatcher().parse(input, source);
                    CommandSyntaxException ex = Commands.getParseException(parse);
                    if (ex != null) {
                        result = "ERR " + ex.getCursor() + " " + message(ex.getRawMessage());
                    } else {
                        result = finish(parse, null);
                    }
                } catch (Throwable t) {
                    result = "CRASH " + t.getClass().getSimpleName() + " " + t.getMessage();
                }
                w.println(result);
            }
        }
    }

    private static String finish(ParseResults<CommandSourceStack> parse, StringBuilder ignored) {
        CommandContextBuilder<CommandSourceStack> c = parse.getContext();
        StringBuilder nodes = new StringBuilder();
        boolean first = true;
        CommandContextBuilder<CommandSourceStack> last = c;
        while (c != null) {
            if (!first) nodes.append('>');
            first = false;
            boolean firstNode = true;
            for (ParsedCommandNode<CommandSourceStack> n : c.getNodes()) {
                if (!firstNode) nodes.append('/');
                firstNode = false;
                nodes.append(n.getNode().getName());
            }
            last = c;
            c = c.getChild();
        }
        if (last.getCommand() == null) {
            CommandSyntaxException e = CommandSyntaxException.BUILT_IN_EXCEPTIONS.dispatcherUnknownCommand()
                .createWithContext(parse.getReader());
            return "ERR " + e.getCursor() + " " + message(e.getRawMessage());
        }
        return "OK " + nodes + " 1";
    }

    private static String message(com.mojang.brigadier.Message m) {
        if (m instanceof Component c) {
            JsonElement json = ComponentSerialization.CODEC.encodeStart(JsonOps.INSTANCE, c).getOrThrow();
            return json.toString();
        }
        return "lit:" + m.getString();
    }
}
