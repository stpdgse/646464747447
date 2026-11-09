package com.openfpv.client;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGameOver;
import net.minecraft.client.gui.GuiIngameMenu;
import net.minecraft.entity.EntityLivingBase;
import net.minecraft.entity.monster.EntityCreeper;
import net.minecraft.entity.monster.EntityGolem;
import net.minecraft.entity.monster.EntityMagmaCube;
import net.minecraft.entity.monster.EntitySkeleton;
import net.minecraft.entity.monster.EntitySlime;
import net.minecraft.entity.monster.EntityZombie;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraftforge.client.event.EntityViewRenderEvent;
import net.minecraftforge.client.event.GuiOpenEvent;
import net.minecraftforge.client.event.InputUpdateEvent;
import net.minecraftforge.client.event.MouseEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.client.event.RenderHandEvent;
import net.minecraftforge.client.event.RenderLivingEvent;
import net.minecraftforge.event.world.WorldEvent;
import net.minecraftforge.fml.common.eventhandler.EventPriority;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import net.minecraftforge.fml.common.network.FMLNetworkEvent;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

@SideOnly(Side.CLIENT)
public class FpvEvents {

    @SubscribeEvent
    public void onClientTick(TickEvent.ClientTickEvent event) {
        if (event.phase == TickEvent.Phase.END) FpvClient.clientTick();
    }

    @SubscribeEvent
    public void onRenderTick(TickEvent.RenderTickEvent event) {
        if (event.phase == TickEvent.Phase.START) FpvClient.aimCamera(event.renderTickTime);
    }

    /** Esc leaves the FPV view instead of opening the pause menu. */
    @SubscribeEvent(priority = EventPriority.HIGHEST)
    public void onGuiOpen(GuiOpenEvent event) {
        if (!FpvClient.isViewing()) return;
        if (event.getGui() instanceof GuiIngameMenu) {
            event.setCanceled(true);
            FpvClient.exit(true, null);
        } else if (event.getGui() instanceof GuiGameOver) {
            FpvClient.exit(true, null);
        }
    }

    /** The pilot's body stays put; movement keys fly the drone instead. */
    @SubscribeEvent
    public void onInput(InputUpdateEvent event) {
        if (!FpvClient.isViewing()) return;
        net.minecraft.util.MovementInput in = event.getMovementInput();
        in.moveForward = 0f;
        in.moveStrafe = 0f;
        in.jump = false;
        in.sneak = false;
        in.forwardKeyDown = in.backKeyDown = in.leftKeyDown = in.rightKeyDown = false;
    }

    @SubscribeEvent(priority = EventPriority.HIGHEST)
    public void onMouse(MouseEvent event) {
        if (FpvClient.isViewing() && (event.getButton() >= 0 || event.getDwheel() != 0)) {
            event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onFov(EntityViewRenderEvent.FOVModifier event) {
        FpvClient.Cam cam = FpvClient.current();
        if (cam != null) event.setFOV(cam.fov);
    }

    @SubscribeEvent
    public void onRenderHand(RenderHandEvent event) {
        if (FpvClient.isViewing()) event.setCanceled(true);
    }

    @SubscribeEvent(priority = EventPriority.HIGH)
    public void onOverlayPre(RenderGameOverlayEvent.Pre event) {
        if (!FpvClient.isViewing()) return;
        switch (event.getType()) {
            case ALL:
            case TEXT:
            case CHAT:
            case DEBUG:
            case SUBTITLES:
                break;
            default:
                event.setCanceled(true);
        }
    }

    @SubscribeEvent
    public void onOverlayPost(RenderGameOverlayEvent.Post event) {
        if (FpvClient.isViewing() && event.getType() == RenderGameOverlayEvent.ElementType.ALL) {
            Hud.render(event.getResolution(), event.getPartialTicks());
        }
    }

    // ---- thermal vision: living things are drawn flat and hot; cold-blooded ones are cooler

    private static float heat(EntityLivingBase e) {
        if (e instanceof EntityCreeper || e instanceof EntitySkeleton || e instanceof EntityZombie
            || e instanceof EntityGolem) return 0.42f;
        if (e instanceof EntitySlime && !(e instanceof EntityMagmaCube)) return 0.55f;
        if (e instanceof EntityMagmaCube) return 1.0f;
        return 0.92f;
    }

    private static boolean thermal() {
        FpvClient.Cam c = FpvClient.current();
        return c != null && c.thermal;
    }

    @SubscribeEvent
    public void onLivingPre(RenderLivingEvent.Pre<EntityLivingBase> event) {
        if (!thermal() || event.getEntity() == Minecraft.getMinecraft().getRenderViewEntity()) return;
        float h = heat(event.getEntity());
        GlStateManager.disableLighting();
        GlStateManager.disableTexture2D();
        GlStateManager.color(h, h, h, 1f);
    }

    @SubscribeEvent
    public void onLivingPost(RenderLivingEvent.Post<EntityLivingBase> event) {
        if (!thermal() || event.getEntity() == Minecraft.getMinecraft().getRenderViewEntity()) return;
        GlStateManager.enableTexture2D();
        GlStateManager.enableLighting();
        GlStateManager.color(1f, 1f, 1f, 1f);
    }

    // ---- housekeeping

    @SubscribeEvent
    public void onWorldUnload(WorldEvent.Unload event) {
        if (event.getWorld().isRemote) FpvClient.reset();
    }

    @SubscribeEvent
    public void onDisconnect(FMLNetworkEvent.ClientDisconnectionFromServerEvent event) {
        FpvClient.reset();
    }
}
