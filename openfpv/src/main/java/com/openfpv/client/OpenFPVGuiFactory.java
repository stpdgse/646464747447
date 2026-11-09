package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.OpenFPV;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiScreen;
import net.minecraftforge.common.config.ConfigElement;
import net.minecraftforge.fml.client.DefaultGuiFactory;
import net.minecraftforge.fml.client.IModGuiFactory;
import net.minecraftforge.fml.client.config.GuiConfig;
import net.minecraftforge.fml.client.config.IConfigElement;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/** Mods -> OpenFPV -> Config: edits config/openfpv.cfg in game; changes apply immediately. */
@SideOnly(Side.CLIENT)
public class OpenFPVGuiFactory implements IModGuiFactory {
    @Override
    public void initialize(Minecraft minecraftInstance) {}

    @Override
    public boolean hasConfigGui() {
        return true;
    }

    @Override
    public GuiScreen createConfigGui(GuiScreen parent) {
        List<IConfigElement> elements = new ArrayList<>();
        for (String name : Config.cfg.getCategoryNames()) {
            elements.add(new ConfigElement(Config.cfg.getCategory(name)));
        }
        return new GuiConfig(parent, elements, OpenFPV.MOD_ID, false, false, "OpenFPV");
    }

    @Override
    public Set<IModGuiFactory.RuntimeOptionCategoryElement> runtimeGuiCategories() {
        return null;
    }
}
