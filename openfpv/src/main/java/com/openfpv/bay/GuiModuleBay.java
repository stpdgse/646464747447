package com.openfpv.bay;

import com.openfpv.ModItems;
import com.openfpv.OpenFPV;
import com.openfpv.item.ModuleType;
import net.minecraft.client.gui.inventory.GuiContainer;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.RenderHelper;
import net.minecraft.client.resources.I18n;
import net.minecraft.inventory.Slot;
import net.minecraft.item.ItemStack;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

@SideOnly(Side.CLIENT)
public class GuiModuleBay extends GuiContainer {
    private static final ResourceLocation TEXTURE = new ResourceLocation(OpenFPV.MOD_ID, "textures/gui/module_bay.png");

    public GuiModuleBay(ContainerModuleBay container) {
        super(container);
        xSize = 176;
        ySize = 133;
    }

    @Override
    public void drawScreen(int mouseX, int mouseY, float partialTicks) {
        drawDefaultBackground();
        super.drawScreen(mouseX, mouseY, partialTicks);
        renderHoveredToolTip(mouseX, mouseY);
    }

    @Override
    protected void drawGuiContainerBackgroundLayer(float partialTicks, int mouseX, int mouseY) {
        GlStateManager.color(1f, 1f, 1f, 1f);
        mc.getTextureManager().bindTexture(TEXTURE);
        drawTexturedModalRect(guiLeft, guiTop, 0, 0, xSize, ySize);
    }

    @Override
    protected void drawGuiContainerForegroundLayer(int mouseX, int mouseY) {
        fontRenderer.drawString(I18n.format("container.openfpv.module_bay"), 8, 6, 0x404040);
        fontRenderer.drawString(I18n.format("container.inventory"), 8, 40, 0x404040);
        // Ghost icons in empty bay slots show which module belongs where.
        RenderHelper.enableGUIStandardItemLighting();
        for (int i = 0; i < ModuleBay.SIZE; i++) {
            Slot slot = inventorySlots.getSlot(i);
            if (!slot.getHasStack()) {
                ItemStack ghost = new ItemStack(ModItems.MODULES[i]);
                itemRender.renderItemIntoGUI(ghost, slot.xPos, slot.yPos);
                GlStateManager.disableLighting();
                GlStateManager.disableDepth();
                drawRect(slot.xPos, slot.yPos, slot.xPos + 16, slot.yPos + 16, 0xB08B8B8B);
                GlStateManager.enableDepth();
            }
        }
    }

    @Override
    protected void renderHoveredToolTip(int x, int y) {
        Slot hovered = getSlotUnderMouse();
        if (hovered != null && hovered.slotNumber < ModuleBay.SIZE && !hovered.getHasStack()) {
            ModuleType t = ModuleType.VALUES[hovered.slotNumber];
            drawHoveringText(I18n.format("item.openfpv." + t.itemName + ".name"), x, y);
        } else {
            super.renderHoveredToolTip(x, y);
        }
    }
}
