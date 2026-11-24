import net.minecraft.core.BlockPos;
import java.io.PrintWriter;
import java.util.*;

public final class HashOrder {
    public static void main(String[] a) throws Exception {
        Random rng = new Random(12345);
        try (PrintWriter w = new PrintWriter(a[0])) {
            w.println("# each line: insertion order | java HashSet iteration order; positions as x,y,z");
            for (int n = 0; n < 400; n++) {
                int size = 1 + rng.nextInt(12);
                int spread = 4 + rng.nextInt(30);
                int bx = rng.nextInt(2000) - 1000, by = rng.nextInt(300) - 64, bz = rng.nextInt(2000) - 1000;
                List<BlockPos> ins = new ArrayList<>();
                Set<BlockPos> set = new HashSet<>();
                while (ins.size() < size) {
                    BlockPos p = new BlockPos(bx + rng.nextInt(spread), by + rng.nextInt(Math.min(spread, 20)), bz + rng.nextInt(spread));
                    if (set.add(p)) ins.add(p);
                }
                StringBuilder sb = new StringBuilder();
                for (BlockPos p : ins) sb.append(p.getX()).append(',').append(p.getY()).append(',').append(p.getZ()).append(' ');
                sb.append('|');
                for (BlockPos p : set) sb.append(' ').append(p.getX()).append(',').append(p.getY()).append(',').append(p.getZ());
                w.println(sb);
            }
        }
    }
}
