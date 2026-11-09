package com.openfpv;

import com.openfpv.item.ModuleType;
import li.cil.oc.api.Items;
import li.cil.oc.api.detail.ItemInfo;
import net.minecraft.item.ItemStack;
import net.minecraft.item.crafting.IRecipe;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.event.RegistryEvent;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.oredict.ShapedOreRecipe;
import org.apache.logging.log4j.LogManager;

/**
 * Recipes use OpenComputers parts so the camera sits at a sensible point in an OC-centred
 * progression (chips, PCBs, graphics card). They are built in code because OC part ids are only
 * known at runtime; if one is missing a vanilla stand-in is used and a warning is logged.
 */
@Mod.EventBusSubscriber(modid = OpenFPV.MOD_ID)
public final class ModRecipes {
    private ModRecipes() {}

    /** An OC part by its API name, or a vanilla stand-in if OC does not know it. */
    private static Object oc(String name, Object fallback) {
        ItemInfo info = li.cil.oc.api.Items.get(name);
        if (info != null) {
            ItemStack s = info.createItemStack(1);
            if (!s.isEmpty()) return s;
        }
        LogManager.getLogger(OpenFPV.MOD_ID).warn("OpenComputers part '{}' not found; using a stand-in in recipes", name);
        return fallback;
    }

    @SubscribeEvent
    public static void registerRecipes(RegistryEvent.Register<IRecipe> event) {
        Object chip1 = oc("chip1", "ingotGold");
        Object chip2 = oc("chip2", "gemDiamond");
        Object pcb = oc("printedcircuitboard", "ingotIron");
        Object gpu = oc("graphicscard1", chip1);

        add(event, "camera_upgrade", new ItemStack(ModItems.CAMERA),
            "IGI", "RVR", "IPI",
            'I', "ingotIron", 'G', "paneGlass", 'R', "dustRedstone", 'V', gpu, 'P', pcb);

        add(event, ModuleType.NIGHT_VISION, "DGD", "QCQ", "IPI",
            'D', "dustGlowstone", 'G', "paneGlass", 'Q', "gemQuartz", 'C', chip2, 'I', "ingotIron", 'P', pcb);
        add(event, ModuleType.ZOOM, "GAG", "ICI", "IPI",
            'G', "paneGlass", 'A', "ingotGold", 'C', chip1, 'I', "ingotIron", 'P', pcb);
        add(event, ModuleType.GIMBAL, "IXI", "RCR", "IPI",
            'X', net.minecraft.init.Blocks.PISTON, 'R', "dustRedstone", 'C', chip1, 'I', "ingotIron", 'P', pcb);
        add(event, ModuleType.THERMAL, "BQB", "GCG", "IPI",
            'B', net.minecraft.init.Items.BLAZE_POWDER, 'Q', "gemQuartz", 'G', "paneGlass", 'C', chip2,
            'I', "ingotIron", 'P', pcb);
        add(event, ModuleType.DEPTH, "ERE", "LCL", "IPI",
            'E', net.minecraft.init.Items.ENDER_PEARL, 'R', "dustRedstone", 'L', "gemLapis", 'C', chip2,
            'I', "ingotIron", 'P', pcb);
    }

    private static void add(RegistryEvent.Register<IRecipe> event, ModuleType type, Object... recipe) {
        add(event, type.itemName, new ItemStack(ModItems.MODULES[type.ordinal()]), recipe);
    }

    private static void add(RegistryEvent.Register<IRecipe> event, String name, ItemStack result, Object... recipe) {
        ResourceLocation id = new ResourceLocation(OpenFPV.MOD_ID, name);
        ShapedOreRecipe r = new ShapedOreRecipe(id, result, recipe);
        r.setRegistryName(id);
        event.getRegistry().register(r);
    }
}
