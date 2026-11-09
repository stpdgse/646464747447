package com.openfpv;

import com.openfpv.item.ItemAntenna;
import com.openfpv.item.ItemCamera;
import com.openfpv.item.ItemModule;
import com.openfpv.item.ModuleType;
import net.minecraft.client.renderer.block.model.ModelResourceLocation;
import net.minecraft.creativetab.CreativeTabs;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraftforge.client.event.ModelRegistryEvent;
import net.minecraftforge.client.model.ModelLoader;
import net.minecraftforge.event.RegistryEvent;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

@Mod.EventBusSubscriber(modid = OpenFPV.MOD_ID)
public final class ModItems {
    private ModItems() {}

    public static final CreativeTabs TAB = new CreativeTabs("openfpv") {
        @Override
        public ItemStack getTabIconItem() {
            return new ItemStack(CAMERA);
        }
    };

    public static final ItemCamera CAMERA = new ItemCamera();
    public static final ItemModule[] MODULES = new ItemModule[ModuleType.VALUES.length];
    /** Antenna Mk1..Mk4 (index 0 = Mk1). */
    public static final ItemAntenna[] ANTENNAS = new ItemAntenna[4];

    static {
        name(CAMERA, "camera_upgrade");
        for (ModuleType t : ModuleType.VALUES) {
            ItemModule m = new ItemModule(t);
            name(m, t.itemName);
            MODULES[t.ordinal()] = m;
        }
        for (int i = 0; i < ANTENNAS.length; i++) {
            ANTENNAS[i] = new ItemAntenna(i + 1);
            name(ANTENNAS[i], "antenna_mk" + (i + 1));
        }
    }

    private static void name(Item item, String path) {
        item.setRegistryName(OpenFPV.MOD_ID, path);
        item.setUnlocalizedName(OpenFPV.MOD_ID + "." + path);
        item.setCreativeTab(TAB);
    }

    @SubscribeEvent
    public static void registerItems(RegistryEvent.Register<Item> event) {
        event.getRegistry().register(CAMERA);
        event.getRegistry().registerAll(MODULES);
        event.getRegistry().registerAll(ANTENNAS);
    }

    @SubscribeEvent
    @SideOnly(Side.CLIENT)
    public static void registerModels(ModelRegistryEvent event) {
        ModelLoader.setCustomModelResourceLocation(CAMERA, 0, new ModelResourceLocation(CAMERA.getRegistryName(), "inventory"));
        for (ItemModule m : MODULES) {
            ModelLoader.setCustomModelResourceLocation(m, 0, new ModelResourceLocation(m.getRegistryName(), "inventory"));
        }
        for (ItemAntenna a : ANTENNAS) {
            ModelLoader.setCustomModelResourceLocation(a, 0, new ModelResourceLocation(a.getRegistryName(), "inventory"));
        }
    }
}
