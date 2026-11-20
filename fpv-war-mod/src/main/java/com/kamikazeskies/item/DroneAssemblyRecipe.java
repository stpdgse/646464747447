package com.kamikazeskies.item;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.build.PartKind;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.Warhead;
import com.mojang.serialization.MapCodec;

import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.crafting.CraftingInput;
import net.minecraft.world.item.crafting.CustomRecipe;
import net.minecraft.world.item.crafting.RecipeSerializer;
import net.minecraft.world.level.Level;

/**
 * Assembling a drone: put one of each required part (frame, motors, props, battery, flight controller, camera, antenna)
 * in a crafting grid, plus optionally one warhead and one blackbox. Nothing else may be in the grid. The result is a
 * drone item whose stats come from the tiers of the parts used.
 */
public class DroneAssemblyRecipe extends CustomRecipe {
    public static final MapCodec<DroneAssemblyRecipe> MAP_CODEC = MapCodec.unit(DroneAssemblyRecipe::new);
    public static final StreamCodec<RegistryFriendlyByteBuf, DroneAssemblyRecipe> STREAM_CODEC = StreamCodec.unit(new DroneAssemblyRecipe());
    public static final RecipeSerializer<DroneAssemblyRecipe> SERIALIZER = new RecipeSerializer<>(MAP_CODEC, STREAM_CODEC);

    @Override
    public RecipeSerializer<? extends CustomRecipe> getSerializer() { return SERIALIZER; }

    @Override
    public boolean matches(CraftingInput input, Level level) { return parse(input) != null; }

    @Override
    public ItemStack assemble(CraftingInput input) {
        DroneBuild build = parse(input);
        ItemStack out = new ItemStack(KsRegistries.DRONE_ITEM.get());
        if (build != null) out.set(KsComponents.DRONE_BUILD.get(), build);
        return out;
    }

    /** @return the build the grid describes, or {@code null} if it is not a valid assembly */
    public static @Nullable DroneBuild parse(CraftingInput input) {
        int[] tier = new int[PartKind.values().length];
        int[] count = new int[PartKind.values().length];
        for (int i = 0; i < input.size(); i++) {
            ItemStack s = input.getItem(i);
            if (s.isEmpty()) continue;
            PartItem p = PartItem.of(s);
            if (p == null || s.getCount() != 1) return null;                      // anything that is not a part spoils the assembly
            count[p.kind().ordinal()]++;
            tier[p.kind().ordinal()] = p.tier();
        }
        for (PartKind k : PartKind.values()) {
            if (count[k.ordinal()] > 1) return null;
            if (k.tiered && count[k.ordinal()] != 1) return null;                 // every tiered part is required
        }
        if (count[PartKind.WARHEAD_IMPACT.ordinal()] + count[PartKind.WARHEAD_SHAPED.ordinal()] > 1) return null;
        Warhead w = count[PartKind.WARHEAD_IMPACT.ordinal()] == 1 ? Warhead.IMPACT : count[PartKind.WARHEAD_SHAPED.ordinal()] == 1 ? Warhead.SHAPED : Warhead.NONE;
        DroneBuild build = new DroneBuild(tier[PartKind.FRAME.ordinal()], tier[PartKind.MOTOR.ordinal()], tier[PartKind.PROP.ordinal()],
                tier[PartKind.BATTERY.ordinal()], tier[PartKind.FLIGHT_CONTROLLER.ordinal()], tier[PartKind.CAMERA.ordinal()], tier[PartKind.ANTENNA.ordinal()],
                w, count[PartKind.BLACKBOX.ordinal()] == 1);
        return build.validate() == null ? build : null;
    }
}
