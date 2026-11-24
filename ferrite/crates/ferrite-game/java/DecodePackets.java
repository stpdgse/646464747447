// Decodes play packets with the official codecs, so Ferrite's encodings can be checked against
// the real thing. Input lines are `<id> <name> <hex body>`; output is one line per packet.
//
//   javac -cp <server jar and libs> DecodePackets.java
//   java  -cp <server jar and libs>:. DecodePackets clientbound|serverbound <file> [per-name limit]
//
// A packet that fails to decode, or leaves bytes unread, is reported as ERROR or TRAILING.
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import net.minecraft.SharedConstants;
import net.minecraft.core.RegistryAccess;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.protocol.Packet;
import net.minecraft.network.protocol.game.GameProtocols;
import net.minecraft.server.Bootstrap;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class DecodePackets {
    static String describe(Object o, int depth) {
        if (o == null) return "null";
        String t = o.toString();
        if (!t.contains("@") || depth > 3) return t;
        StringBuilder sb = new StringBuilder(o.getClass().getSimpleName()).append("{");
        for (var f : o.getClass().getDeclaredFields()) {
            if (java.lang.reflect.Modifier.isStatic(f.getModifiers())) continue;
            try {
                f.setAccessible(true);
                Object v = f.get(o);
                String d = v instanceof byte[] b ? "byte[" + b.length + "]" : describe(v, depth + 1);
                sb.append(f.getName()).append('=').append(d).append(", ");
            } catch (Throwable e) {
                sb.append(f.getName()).append("=?, ");
            }
        }
        return sb.append("}").toString();
    }

    static RegistryAccess registries() {
        try {
            var vanilla = net.minecraft.server.packs.repository.ServerPacksSource.createVanillaPackSource();
            var rm = new net.minecraft.server.packs.resources.MultiPackResourceManager(
                    net.minecraft.server.packs.PackType.SERVER_DATA,
                    List.<net.minecraft.server.packs.PackResources>of(vanilla.fullResources()));
            var staticAccess = RegistryAccess.fromRegistryOfRegistries(net.minecraft.core.registries.BuiltInRegistries.REGISTRY);
            var lookups = net.minecraft.tags.TagLoader.buildUpdatedLookups(staticAccess,
                    net.minecraft.tags.TagLoader.loadTagsForExistingRegistries(rm, staticAccess));
            var dynamic = net.minecraft.resources.RegistryDataLoader.load(rm, lookups,
                    net.minecraft.resources.RegistryDataLoader.WORLD_REGISTRIES, Runnable::run).join();
            return new RegistryAccess.ImmutableRegistryAccess(
                    java.util.stream.Stream.concat(staticAccess.registries(), dynamic.registries())).freeze();
        } catch (Throwable t) {
            System.err.println("registry access fallback to EMPTY: " + t);
            return RegistryAccess.EMPTY;
        }
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        // Item stacks need their default components bound; empty maps are enough for decoding.
        net.minecraft.core.registries.BuiltInRegistries.ITEM.listElements().forEach(h -> {
            if (!h.areComponentsBound()) h.bindComponents(net.minecraft.core.component.DataComponentMap.EMPTY);
        });
        boolean clientbound = args[0].equals("clientbound");
        int limit = args.length > 2 ? Integer.parseInt(args[2]) : Integer.MAX_VALUE;
        var decorator = RegistryFriendlyByteBuf.decorator(registries());
        var codec = clientbound
                ? GameProtocols.CLIENTBOUND_TEMPLATE.bind(decorator).codec()
                : GameProtocols.SERVERBOUND_TEMPLATE.bind(decorator, new GameProtocols.Context() {
                    public boolean hasInfiniteMaterials() { return true; }
                    public boolean canUseCommandBlocks() { return true; }
                }).codec();
        Map<String, Integer> seen = new HashMap<>();
        for (String line : Files.readAllLines(Path.of(args[1]))) {
            if (line.startsWith("#") || line.isBlank()) continue;
            String[] p = line.split(" ");
            int id = Integer.parseInt(p[0]);
            String name = p[1];
            if (seen.merge(name, 1, Integer::sum) > limit) continue;
            byte[] body = new byte[p.length > 2 ? p[2].length() / 2 : 0];
            for (int i = 0; i < body.length; i++) body[i] = (byte) Integer.parseInt(p[2].substring(2 * i, 2 * i + 2), 16);
            ByteBuf buf = Unpooled.buffer();
            net.minecraft.network.VarInt.write(buf, id);
            buf.writeBytes(body);
            String text;
            try {
                Packet<?> pk = codec.decode(buf);
                text = describe(pk, 0);
                if (buf.readableBytes() > 0) text = "TRAILING " + buf.readableBytes() + " bytes: " + text;
            } catch (Throwable t) {
                Throwable c = t;
                while (c.getCause() != null) c = c.getCause();
                text = "ERROR " + c;
            }
            if (text.length() > 400) text = text.substring(0, 400) + "...";
            System.out.println("PACKET " + name + " (" + body.length + "B): " + text);
        }
    }
}
