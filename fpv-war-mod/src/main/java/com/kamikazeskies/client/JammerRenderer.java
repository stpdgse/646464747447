package com.kamikazeskies.client;

import com.geckolib.model.DefaultedEntityGeoModel;
import com.geckolib.renderer.GeoEntityRenderer;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.drone.JammerEntity;

import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.resources.Identifier;

/** Renders the jammer mast from its Blockbench/GeckoLib model. */
public class JammerRenderer extends GeoEntityRenderer<JammerEntity, EntityRenderState> {
    public JammerRenderer(EntityRendererProvider.Context context) {
        super(context, new DefaultedEntityGeoModel<>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "jammer")));
    }
}
