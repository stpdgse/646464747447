package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.item.ModuleType;
import com.openfpv.util.CameraMath;
import java.util.Random;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.FontRenderer;
import net.minecraft.client.gui.ScaledResolution;
import net.minecraft.client.renderer.BufferBuilder;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.client.resources.I18n;
import net.minecraft.entity.Entity;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;
import org.lwjgl.opengl.GL11;

/** The goggle overlay: vignette, crosshair, compass, telemetry and analogue static. */
@SideOnly(Side.CLIENT)
public final class Hud {
    private Hud() {}

    private static final Random RNG = new Random();
    private static final String[] CARDINALS = {"N", "NE", "E", "SE", "S", "SW", "W", "NW"};

    public static void render(ScaledResolution res, float partialTicks) {
        Minecraft mc = Minecraft.getMinecraft();
        FpvClient.Cam cam = FpvClient.current();
        if (cam == null) return;
        int w = res.getScaledWidth(), h = res.getScaledHeight();
        FontRenderer font = mc.fontRenderer;

        int main = cam.thermal ? 0xFFFFB347 : (cam.night ? 0xFF66FF77 : 0xFF7CFC9A);
        int dim = (main & 0x00FFFFFF) | 0x90000000;

        GlStateManager.pushMatrix();
        GlStateManager.disableLighting();
        GlStateManager.disableDepth();
        GlStateManager.enableBlend();
        GlStateManager.tryBlendFuncSeparate(GL11.GL_SRC_ALPHA, GL11.GL_ONE_MINUS_SRC_ALPHA, 1, 0);
        GlStateManager.disableTexture2D();
        // Vanilla leaves GL_FLAT shading on, which would turn the vignette gradients into flat bands.
        GlStateManager.shadeModel(GL11.GL_SMOOTH);

        vignette(w, h);
        if (Config.noiseEnabled) Noise.render(w, h, FpvClient.quality, FpvClient.viewTicks + partialTicks);

        // crosshair
        int cx = w / 2, cy = h / 2;
        rect(cx - 9, cy, cx - 3, cy + 1, main);
        rect(cx + 3, cy, cx + 9, cy + 1, main);
        rect(cx, cy - 9, cx + 1, cy - 3, main);
        rect(cx, cy + 3, cx + 1, cy + 9, main);
        rect(cx, cy, cx + 1, cy + 1, main);

        horizon(cx, cy, main, dim, partialTicks);
        compass(w, main, dim, font);

        GlStateManager.enableTexture2D();
        telemetry(mc, cam, w, h, font, main, partialTicks);
        GlStateManager.shadeModel(GL11.GL_FLAT); // what vanilla expects
        GlStateManager.disableBlend();
        GlStateManager.enableDepth();
        GlStateManager.popMatrix();
    }

    // ------------------------------------------------------------------ pieces

    private static void telemetry(Minecraft mc, FpvClient.Cam cam, int w, int h, FontRenderer font, int main, float pt) {
        Entity e = mc.getRenderViewEntity();
        double alt = e == null ? 0 : e.posY;
        int x = 10, y = h / 2 - 28;
        font.drawStringWithShadow(I18n.format("hud.openfpv.alt", fmt(alt)), x, y, main);
        font.drawStringWithShadow(I18n.format("hud.openfpv.spd", fmt(FpvClient.speed)), x, y + 11, main);
        font.drawStringWithShadow(I18n.format("hud.openfpv.rng", Math.round(FpvClient.distance), Math.round(cam.range)), x, y + 22, main);

        // battery
        int pct = Math.round(cam.energy * 100f);
        boolean low = pct < 15;
        boolean blink = (mc.world.getTotalWorldTime() / 6) % 2 == 0;
        int pwrColor = low ? (blink ? 0xFFFF4040 : 0xFF802020) : main;
        font.drawStringWithShadow(I18n.format("hud.openfpv.pwr", pct), x, y + 33, pwrColor);
        GlStateManager.disableTexture2D();
        rect(x, y + 44, x + 60, y + 49, 0x66000000);
        rect(x + 1, y + 45, x + 1 + Math.round(58 * cam.energy), y + 48, pwrColor);
        GlStateManager.enableTexture2D();

        // right side: signal bars + optics
        String sig = I18n.format("hud.openfpv.sig");
        int sx = w - 10 - 5 * 7;
        font.drawStringWithShadow(sig, sx - font.getStringWidth(sig) - 4, y, main);
        GlStateManager.disableTexture2D();
        int bars = Math.round(FpvClient.quality * 5f);
        for (int i = 0; i < 5; i++) {
            int bh = 3 + i * 3;
            rect(sx + i * 7, y + 12 - bh, sx + i * 7 + 5, y + 12, i < bars ? main : 0x55FFFFFF);
        }
        GlStateManager.enableTexture2D();
        String fov = I18n.format("hud.openfpv.zoom", fmt(CameraMath.fovToZoom(cam.fov)));
        font.drawStringWithShadow(fov, w - 10 - font.getStringWidth(fov), y + 22, main);
        if (FpvClient.lagTicks > 0) {
            String lag = I18n.format("hud.openfpv.lag", FpvClient.lagTicks * 50);
            font.drawStringWithShadow(lag, w - 10 - font.getStringWidth(lag), y + 44, FpvClient.lagTicks >= 6 ? 0xFFFF6060 : main);
        }
        StringBuilder tags = new StringBuilder();
        if (cam.night) tags.append("[NV] ");
        if (cam.thermal) tags.append("[THERMAL] ");
        if (ModuleType.has(cam.modules, ModuleType.DEPTH)) tags.append("[DEPTH] ");
        if (ModuleType.has(cam.modules, ModuleType.GIMBAL) && (cam.pan != 0f || cam.tilt != 0f)) {
            tags.append(String.format("[GIM %+.0f/%+.0f]", cam.pan, cam.tilt));
        }
        String t = tags.toString().trim();
        if (!t.isEmpty()) font.drawStringWithShadow(t, w - 10 - font.getStringWidth(t), y + 33, main);

        // REC + exit hint
        if ((mc.world.getTotalWorldTime() / 10) % 2 == 0) {
            font.drawStringWithShadow("● REC", 10, 10, 0xFFFF3030);
        }
        String hint = I18n.format("hud.openfpv.hint");
        font.drawStringWithShadow(hint, w / 2 - font.getStringWidth(hint) / 2, h - 16, 0xAAFFFFFF);
        if (FpvClient.quality < 0.25f) {
            String warn = I18n.format("hud.openfpv.weak_signal");
            if ((mc.world.getTotalWorldTime() / 5) % 2 == 0) {
                font.drawStringWithShadow(warn, w / 2 - font.getStringWidth(warn) / 2, h / 2 + 24, 0xFFFF4040);
            }
        }
    }

    /**
     * Artificial horizon: a bar that stays parallel to the real horizon, drawn in the camera's frame, so
     * it rolls and slides with the drone's attitude. The short ticks mark 10 degrees of pitch.
     */
    private static void horizon(int cx, int cy, int main, int dim, float partialTicks) {
        float roll = FpvClient.viewRoll(partialTicks);
        float pitch = FpvClient.viewPitch(partialTicks);
        float ppd = 3.5f; // pixels per degree of pitch
        float y = CameraMath.clamp(-pitch * ppd, -90f, 90f);
        GlStateManager.pushMatrix();
        GlStateManager.translate(cx, cy, 0);
        GlStateManager.rotate(-roll, 0, 0, 1);
        rect(-70, Math.round(y), -22, Math.round(y) + 1, dim);
        rect(22, Math.round(y), 70, Math.round(y) + 1, dim);
        for (int d = -30; d <= 30; d += 10) {
            if (d == 0) continue;
            int ly = Math.round(y - d * ppd);
            rect(-14, ly, 14, ly + 1, (dim & 0x00FFFFFF) | 0x50000000);
        }
        GlStateManager.popMatrix();
    }

    private static void compass(int w, int main, int dim, FontRenderer font) {
        float heading = CameraMath.compassHeading(FpvClient.viewYaw());
        int cx = w / 2, top = 8, half = 110;
        float span = 70f; // degrees visible each side
        rect(cx - half, top + 14, cx + half, top + 15, dim);
        for (int deg = -180; deg <= 540; deg += 5) {
            float rel = deg - heading;
            while (rel < -180f) rel += 360f;
            while (rel > 180f) rel -= 360f;
            if (Math.abs(rel) > span) continue;
            int px = cx + Math.round(rel / span * half);
            int len = (deg % 15 == 0) ? 5 : 2;
            rect(px, top + 14 - len, px + 1, top + 14, dim);
        }
        rect(cx, top + 16, cx + 1, top + 21, main);
        GlStateManager.enableTexture2D();
        for (int i = 0; i < 8; i++) {
            float rel = i * 45f - heading;
            while (rel < -180f) rel += 360f;
            while (rel > 180f) rel -= 360f;
            if (Math.abs(rel) > span) continue;
            int px = cx + Math.round(rel / span * half);
            font.drawStringWithShadow(CARDINALS[i], px - font.getStringWidth(CARDINALS[i]) / 2, top - 2, main);
        }
        String deg = String.format("%03d°", Math.round(heading) % 360);
        font.drawStringWithShadow(deg, cx - font.getStringWidth(deg) / 2, top + 22, main);
        GlStateManager.disableTexture2D();
    }

    private static void vignette(int w, int h) {
        int edge = Math.max(24, h / 6);
        int dark = 0xB0000000, clear = 0x00000000;
        gradient(0, 0, w, edge, dark, dark, clear, clear);
        gradient(0, h - edge, w, h, clear, clear, dark, dark);
        gradient(0, 0, edge, h, dark, clear, clear, dark);
        gradient(w - edge, 0, w, h, clear, dark, dark, clear);
    }

    // ------------------------------------------------------------------ drawing helpers

    private static String fmt(double v) {
        return String.format("%.1f", v);
    }

    static void rect(int x1, int y1, int x2, int y2, int argb) {
        gradient(x1, y1, x2, y2, argb, argb, argb, argb);
    }

    /** Corner colours: top-left, top-right, bottom-right, bottom-left. */
    static void gradient(int x1, int y1, int x2, int y2, int tl, int tr, int br, int bl) {
        Tessellator t = Tessellator.getInstance();
        BufferBuilder b = t.getBuffer();
        b.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_COLOR);
        vertex(b, x1, y1, tl);
        vertex(b, x1, y2, bl);
        vertex(b, x2, y2, br);
        vertex(b, x2, y1, tr);
        t.draw();
    }

    private static void vertex(BufferBuilder b, double x, double y, int argb) {
        b.pos(x, y, 0).color((argb >> 16) & 255, (argb >> 8) & 255, argb & 255, (argb >>> 24) & 255).endVertex();
    }

    /** Analogue-TV style static, scanlines and tearing that grow as the signal weakens. */
    static final class Noise {
        private Noise() {}

        static void render(int w, int h, float quality, float time) {
            float s = 1f - quality; // 0 = perfect, 1 = lost
            if (s <= 0.01f) return;
            RNG.setSeed((long) (time * 7919f) * 31L + 17L);
            Tessellator t = Tessellator.getInstance();
            BufferBuilder b = t.getBuffer();
            b.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_COLOR);

            if (s > 0.85f) { // nearly gone: dim everything
                quad(b, 0, 0, w, h, 0, 0, 0, Math.min(0.75f, (s - 0.85f) * 5f));
            }
            // scanlines
            int lineAlpha = Math.round(70 * s);
            for (int y = (int) (time % 3); y < h; y += 3) {
                quad(b, 0, y, w, y + 1, 0, 0, 0, lineAlpha / 255f);
            }
            // static speckle
            int dots = Math.round(s * s * 2600f + s * 250f);
            for (int i = 0; i < dots; i++) {
                int x = RNG.nextInt(w), y = RNG.nextInt(h);
                int size = 1 + RNG.nextInt(3);
                float g = 0.35f + RNG.nextFloat() * 0.65f;
                quad(b, x, y, x + size * 2, y + size, g, g, g, (0.15f + RNG.nextFloat() * 0.6f) * s);
            }
            // tearing bars
            int bars = Math.round(s * 7f);
            for (int i = 0; i < bars; i++) {
                int y = RNG.nextInt(h), bh = 2 + RNG.nextInt(8);
                float g = 0.55f + RNG.nextFloat() * 0.45f;
                quad(b, 0, y, w, y + bh, g, g, g, (0.1f + RNG.nextFloat() * 0.35f) * s);
            }
            t.draw();
        }

        private static void quad(BufferBuilder b, double x1, double y1, double x2, double y2,
                                 float r, float g, float bl, float a) {
            int ri = (int) (r * 255), gi = (int) (g * 255), bi = (int) (bl * 255), ai = (int) (CameraMath.clamp(a, 0f, 1f) * 255);
            b.pos(x1, y2, 0).color(ri, gi, bi, ai).endVertex();
            b.pos(x2, y2, 0).color(ri, gi, bi, ai).endVertex();
            b.pos(x2, y1, 0).color(ri, gi, bi, ai).endVertex();
            b.pos(x1, y1, 0).color(ri, gi, bi, ai).endVertex();
        }
    }
}
