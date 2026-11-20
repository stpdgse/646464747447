package com.kamikazeskies.client;

import com.geckolib.constant.DataTickets;
import com.geckolib.model.DefaultedEntityGeoModel;
import com.geckolib.renderer.GeoEntityRenderer;
import com.geckolib.renderer.base.GeoRenderState;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.drone.TankEntity;

import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;

/** Renders the tank; a destroyed one switches to the burnt-out wreck texture. */
public class TankRenderer extends GeoEntityRenderer<TankEntity, EntityRenderState> {
    private static final Identifier NORMAL = Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "textures/entity/tank.png");
    private static final Identifier WRECK = Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "textures/entity/tank_wreck.png");

    public TankRenderer(EntityRendererProvider.Context context) {
        super(context, new DefaultedEntityGeoModel<TankEntity>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "tank")) {
            @Override public Identifier getTextureResource(GeoRenderState state) {
                return state.getOrDefaultGeckolibData(DataTickets.IS_DEAD_OR_DYING, false) ? WRECK : NORMAL;
            }
        });
        this.shadowRadius = 1.6f;
    }

    @Override public int getPackedOverlay(TankEntity tank, Void relatedObject, float u, float partialTick) {
        return tank.isDeadOrDying() ? OverlayTexture.NO_OVERLAY : super.getPackedOverlay(tank, relatedObject, u, partialTick);
    }

    @Override protected float getDeathMaxRotation(GeoRenderState renderState) { return 0f; }
}
