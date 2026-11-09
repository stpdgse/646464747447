package com.openfpv.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.BufferBuilder;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.Tessellator;
import net.minecraft.client.renderer.vertex.DefaultVertexFormats;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;
import org.lwjgl.opengl.GL11;

/**
 * Loads a Blockbench ".bbmodel" ("Free Model" format with box UV) and draws it. The model is authored in
 * 1/16-block pixel units with Y up, facing -Z. Elements are grouped by the outliner; each group can be
 * rotated about its origin by an {@link Animator}, which also colours/blanks elements whose name starts with
 * "light_" (those are drawn fullbright).
 */
@SideOnly(Side.CLIENT)
public final class BbModel {
    public interface Animator {
        /** Rotation {x, y, z} in degrees about the group's origin, or null for none. */
        float[] rotation(String group);

        /** RGB colour of a light element, 0 to switch it off, -1 if the element is not a light. */
        int light(String element);
    }

    private static final class Cube {
        String name;
        float[] from, to, origin, rot;
        int u, v;
        boolean light;
    }

    private static final class Group {
        String name;
        float[] origin;
        final List<Cube> cubes = new ArrayList<>();
    }

    private final List<Group> groups = new ArrayList<>();
    private final float texW, texH;

    private BbModel(float texW, float texH) {
        this.texW = texW;
        this.texH = texH;
    }

    private static float[] vec(JsonArray a, float def) {
        float[] v = {def, def, def};
        if (a != null) for (int i = 0; i < 3 && i < a.size(); i++) v[i] = a.get(i).getAsFloat();
        return v;
    }

    public static BbModel load(ResourceLocation location) throws Exception {
        JsonObject root;
        try (InputStreamReader in = new InputStreamReader(
            Minecraft.getMinecraft().getResourceManager().getResource(location).getInputStream(), "UTF-8")) {
            root = new JsonParser().parse(in).getAsJsonObject();
        }
        JsonObject res = root.getAsJsonObject("resolution");
        BbModel m = new BbModel(res.get("width").getAsFloat(), res.get("height").getAsFloat());
        Map<String, Cube> byUuid = new HashMap<>();
        for (JsonElement el : root.getAsJsonArray("elements")) {
            JsonObject e = el.getAsJsonObject();
            if (!"cube".equals(e.get("type").getAsString())) continue;
            Cube c = new Cube();
            c.name = e.get("name").getAsString();
            c.from = vec(e.getAsJsonArray("from"), 0);
            c.to = vec(e.getAsJsonArray("to"), 0);
            c.rot = vec(e.getAsJsonArray("rotation"), 0);
            c.origin = e.has("origin") ? vec(e.getAsJsonArray("origin"), 0)
                : new float[]{(c.from[0] + c.to[0]) / 2, (c.from[1] + c.to[1]) / 2, (c.from[2] + c.to[2]) / 2};
            JsonArray uv = e.getAsJsonArray("uv_offset");
            c.u = uv == null ? 0 : uv.get(0).getAsInt();
            c.v = uv == null ? 0 : uv.get(1).getAsInt();
            c.light = c.name.startsWith("light_");
            byUuid.put(e.get("uuid").getAsString(), c);
        }
        for (JsonElement el : root.getAsJsonArray("outliner")) {
            if (!el.isJsonObject()) continue;
            JsonObject g = el.getAsJsonObject();
            Group group = new Group();
            group.name = g.get("name").getAsString();
            group.origin = vec(g.getAsJsonArray("origin"), 0);
            for (JsonElement ch : g.getAsJsonArray("children")) {
                Cube c = ch.isJsonPrimitive() ? byUuid.get(ch.getAsString()) : null;
                if (c != null) group.cubes.add(c);
            }
            m.groups.add(group);
        }
        return m;
    }

    public int cubeCount() {
        int n = 0;
        for (Group g : groups) n += g.cubes.size();
        return n;
    }

    public boolean hasGroup(String name) {
        for (Group g : groups) if (g.name.equals(name)) return true;
        return false;
    }

    /** Draws the model at the current GL origin (feet), in block units. The texture must already be bound. */
    public void render(Animator anim, int brightness) {
        GlStateManager.pushMatrix();
        GlStateManager.scale(1f / 16f, 1f / 16f, 1f / 16f);
        GlStateManager.disableCull();
        for (Group g : groups) {
            GlStateManager.pushMatrix();
            float[] r = anim.rotation(g.name);
            if (r != null) {
                GlStateManager.translate(g.origin[0], g.origin[1], g.origin[2]);
                GlStateManager.rotate(r[2], 0, 0, 1);
                GlStateManager.rotate(r[1], 0, 1, 0);
                GlStateManager.rotate(r[0], 1, 0, 0);
                GlStateManager.translate(-g.origin[0], -g.origin[1], -g.origin[2]);
            }
            for (Cube c : g.cubes) {
                if (c.light) {
                    int rgb = anim.light(c.name);
                    GlStateManager.disableLighting();
                    net.minecraft.client.renderer.OpenGlHelper.setLightmapTextureCoords(
                        net.minecraft.client.renderer.OpenGlHelper.lightmapTexUnit, 240f, 240f);
                    if (rgb == 0) {
                        GlStateManager.color(0.12f, 0.12f, 0.12f, 1f);
                    } else {
                        GlStateManager.color(((rgb >> 16) & 255) / 255f, ((rgb >> 8) & 255) / 255f, (rgb & 255) / 255f, 1f);
                    }
                    drawCube(c);
                    GlStateManager.color(1f, 1f, 1f, 1f);
                    GlStateManager.enableLighting();
                    net.minecraft.client.renderer.OpenGlHelper.setLightmapTextureCoords(
                        net.minecraft.client.renderer.OpenGlHelper.lightmapTexUnit, brightness % 65536, brightness / 65536);
                } else {
                    drawCube(c);
                }
            }
            GlStateManager.popMatrix();
        }
        GlStateManager.enableCull();
        GlStateManager.popMatrix();
    }

    private void drawCube(Cube c) {
        boolean rotated = c.rot[0] != 0 || c.rot[1] != 0 || c.rot[2] != 0;
        if (rotated) {
            GlStateManager.pushMatrix();
            GlStateManager.translate(c.origin[0], c.origin[1], c.origin[2]);
            GlStateManager.rotate(c.rot[2], 0, 0, 1);
            GlStateManager.rotate(c.rot[1], 0, 1, 0);
            GlStateManager.rotate(c.rot[0], 1, 0, 0);
            GlStateManager.translate(-c.origin[0], -c.origin[1], -c.origin[2]);
        }
        float x0 = c.from[0], y0 = c.from[1], z0 = c.from[2], x1 = c.to[0], y1 = c.to[1], z1 = c.to[2];
        float w = Math.max(1, (float) Math.ceil(x1 - x0)), h = Math.max(1, (float) Math.ceil(y1 - y0)), d = Math.max(1, (float) Math.ceil(z1 - z0));
        float u = c.u, v = c.v;
        Tessellator t = Tessellator.getInstance();
        BufferBuilder b = t.getBuffer();
        b.begin(GL11.GL_QUADS, DefaultVertexFormats.POSITION_TEX_NORMAL);
        // box-UV layout: up (u+d, v), down (u+d+w, v), east (u, v+d), north (u+d, v+d), west (u+d+w, v+d), south (u+2d+w, v+d)
        face(b, x0, y1, z0, x1, y1, z0, x1, y1, z1, x0, y1, z1, u + d, v, w, d, 0, 1, 0);                  // up
        face(b, x0, y0, z1, x1, y0, z1, x1, y0, z0, x0, y0, z0, u + d + w, v, w, d, 0, -1, 0);             // down
        face(b, x1, y1, z0, x0, y1, z0, x0, y0, z0, x1, y0, z0, u + d, v + d, w, h, 0, 0, -1);             // north
        face(b, x0, y1, z1, x1, y1, z1, x1, y0, z1, x0, y0, z1, u + 2 * d + w, v + d, w, h, 0, 0, 1);      // south
        face(b, x1, y1, z1, x1, y1, z0, x1, y0, z0, x1, y0, z1, u, v + d, d, h, 1, 0, 0);                  // east
        face(b, x0, y1, z0, x0, y1, z1, x0, y0, z1, x0, y0, z0, u + d + w, v + d, d, h, -1, 0, 0);         // west
        t.draw();
        if (rotated) GlStateManager.popMatrix();
    }

    private void face(BufferBuilder b, float ax, float ay, float az, float bx, float by, float bz,
                      float cx, float cy, float cz, float dx, float dy, float dz,
                      float u, float v, float w, float h, float nx, float ny, float nz) {
        float u0 = u / texW, u1 = (u + w) / texW, v0 = v / texH, v1 = (v + h) / texH;
        b.pos(ax, ay, az).tex(u0, v0).normal(nx, ny, nz).endVertex();
        b.pos(bx, by, bz).tex(u1, v0).normal(nx, ny, nz).endVertex();
        b.pos(cx, cy, cz).tex(u1, v1).normal(nx, ny, nz).endVertex();
        b.pos(dx, dy, dz).tex(u0, v1).normal(nx, ny, nz).endVertex();
    }
}
