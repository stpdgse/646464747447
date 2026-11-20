package com.kamikazeskies.client;

import com.geckolib.model.DefaultedEntityGeoModel;
import com.geckolib.renderer.GeoEntityRenderer;
import com.geckolib.renderer.base.GeoRenderState;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.drone.SoldierEntity;

import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.resources.Identifier;

/** Renders the rifleman. The death fall is part of the authored animation, so the vanilla death tilt is disabled. */
public class SoldierRenderer extends GeoEntityRenderer<SoldierEntity, EntityRenderState> {
    public SoldierRenderer(EntityRendererProvider.Context context) {
        super(context, new DefaultedEntityGeoModel<>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "rifleman")));
        this.shadowRadius = 0.4f;
    }

    /** Vanilla tints a dying entity red for its whole death; here only the hit itself flashes, and the body keeps its own colours. */
    @Override public int getPackedOverlay(SoldierEntity soldier, Void relatedObject, float u, float partialTick) {
        return soldier.isDeadOrDying() ? OverlayTexture.NO_OVERLAY : super.getPackedOverlay(soldier, relatedObject, u, partialTick);
    }

    @Override protected float getDeathMaxRotation(GeoRenderState renderState) { return 0f; }
}
