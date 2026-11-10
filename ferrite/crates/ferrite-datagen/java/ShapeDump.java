// Dumps per-block-state shape and property facts from the vanilla jar's own registry code.
//
// This is a data extractor in the same spirit as the vanilla data generator the rest of
// ferrite-datagen drives: it asks the game's registries for facts (boxes, flags) and writes a
// text table. No vanilla logic is copied; the Rust side implements its own behaviour on top.
//
//   javac -cp <server jar and libs> ShapeDump.java
//   java  -cp <server jar and libs>:. ShapeDump <out file>
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.SupportType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.Fluid;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.shapes.VoxelShape;

import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

public final class ShapeDump {
    static final Map<String, Integer> SHAPES = new LinkedHashMap<>();
    static final List<String> SHAPE_LIST = new ArrayList<>();

    static int shapeId(VoxelShape shape) {
        StringBuilder sb = new StringBuilder();
        List<AABB> boxes = new ArrayList<>(shape.toAabbs());
        // Canonical order so equal shapes share an id.
        boxes.sort((a, b) -> {
            int c = Double.compare(a.minY, b.minY);
            if (c != 0) return c;
            c = Double.compare(a.minX, b.minX);
            if (c != 0) return c;
            c = Double.compare(a.minZ, b.minZ);
            if (c != 0) return c;
            c = Double.compare(a.maxY, b.maxY);
            if (c != 0) return c;
            c = Double.compare(a.maxX, b.maxX);
            if (c != 0) return c;
            return Double.compare(a.maxZ, b.maxZ);
        });
        for (AABB b : boxes) {
            sb.append(String.format(Locale.ROOT, "%s,%s,%s,%s,%s,%s;",
                    fmt(b.minX), fmt(b.minY), fmt(b.minZ), fmt(b.maxX), fmt(b.maxY), fmt(b.maxZ)));
        }
        String key = sb.toString();
        Integer id = SHAPES.get(key);
        if (id == null) {
            id = SHAPE_LIST.size();
            SHAPES.put(key, id);
            SHAPE_LIST.add(key);
        }
        return id;
    }

    // Shapes use exact binary fractions in practice; print with full precision so nothing is lost.
    static String fmt(double d) {
        return Double.toString(d);
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Bootstrap.validate();

        Direction[] dirs = Direction.values(); // down, up, north, south, west, east
        int count = 0;
        List<String> rows = new ArrayList<>();
        for (BlockState s : Block.BLOCK_STATE_REGISTRY) {
            int id = Block.BLOCK_STATE_REGISTRY.getId(s);
            if (id != count) throw new IllegalStateException("non contiguous ids at " + id);
            count++;
            var g = EmptyBlockGetter.INSTANCE;
            var p = BlockPos.ZERO;
            Fluid fluid = s.getFluidState().getType();
            int fluidAmount = s.getFluidState().isEmpty() ? 0 : s.getFluidState().getAmount();
            boolean fluidSource = s.getFluidState().isSource();
            boolean fluidFalling = !s.getFluidState().isEmpty()
                    && s.getFluidState().getValue(net.minecraft.world.level.material.FlowingFluid.FALLING);
            StringBuilder sb = new StringBuilder();
            sb.append(id);
            int flags = 0;
            if (s.isAir()) flags |= 1;
            if (s.isSolid()) flags |= 1 << 1;
            if (s.canOcclude()) flags |= 1 << 2;
            if (s.isRedstoneConductor(g, p)) flags |= 1 << 3;
            if (s.isSolidRender()) flags |= 1 << 4;
            if (s.canBeReplaced()) flags |= 1 << 5;
            if (s.propagatesSkylightDown()) flags |= 1 << 6;
            if (s.ignitedByLava()) flags |= 1 << 7;
            if (s.liquid()) flags |= 1 << 8;
            if (s.isSignalSource()) flags |= 1 << 9;
            if (s.hasAnalogOutputSignal()) flags |= 1 << 10;
            if (s.isCollisionShapeFullBlock(g, p)) flags |= 1 << 11;
            if (s.useShapeForLightOcclusion()) flags |= 1 << 12;
            if (s.isRandomlyTicking()) flags |= 1 << 13;
            if (s.hasBlockEntity()) flags |= 1 << 14;
            if (s.getDestroySpeed(g, p) == -1.0f) flags |= 1 << 15;
            sb.append(' ').append(flags);
            sb.append(' ').append(s.getLightEmission());
            sb.append(' ').append(s.getLightDampening());
            sb.append(' ').append(s.getPistonPushReaction().ordinal());
            sb.append(' ').append(net.minecraft.core.registries.BuiltInRegistries.FLUID.getId(fluid));
            sb.append(' ').append(fluidAmount).append(' ').append(fluidSource ? 1 : 0)
                    .append(' ').append(fluidFalling ? 1 : 0);
            sb.append(' ').append(shapeId(s.getCollisionShape(g, p)));
            sb.append(' ').append(shapeId(s.getShape(g, p)));
            sb.append(' ').append(shapeId(s.getOcclusionShape()));
            for (Direction d : dirs) sb.append(' ').append(shapeId(s.getFaceOcclusionShape(d)));
            int sturdy = 0;
            SupportType[] types = {SupportType.FULL, SupportType.CENTER, SupportType.RIGID};
            for (int t = 0; t < 3; t++) {
                for (int i = 0; i < 6; i++) {
                    if (s.isFaceSturdy(g, p, dirs[i], types[t])) sturdy |= 1 << (t * 6 + i);
                }
            }
            sb.append(' ').append(sturdy);
            rows.add(sb.toString());
        }
        try (PrintWriter w = new PrintWriter(args[0], StandardCharsets.UTF_8)) {
            w.println("# direction order: down up north south west east");
            w.println("shapes " + SHAPE_LIST.size());
            for (String s : SHAPE_LIST) w.println(s.isEmpty() ? "-" : s);
            w.println("states " + rows.size());
            for (String r : rows) w.println(r);
        }
        System.out.println("states=" + rows.size() + " shapes=" + SHAPE_LIST.size());
    }
}
