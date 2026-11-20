package com.kamikazeskies.client;

import org.joml.Matrix4f;
import org.joml.Quaternionf;

import com.geckolib.constant.dataticket.DataTicket;
import com.geckolib.model.DefaultedEntityGeoModel;
import com.geckolib.renderer.GeoEntityRenderer;
import com.geckolib.renderer.base.GeoRenderState;
import com.geckolib.renderer.base.RenderPassInfo;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.sim.Quat;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;

import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.resources.Identifier;

/**
 * Renders the drone from its Blockbench/GeckoLib model with the full 3D flight orientation
 * (vanilla entities only support yaw/pitch, which cannot show rolls or flips).
 */
public class DroneRenderer extends GeoEntityRenderer<DroneEntity, EntityRenderState> {

    private static final DataTicket<Quat> ORIENTATION = DataTicket.create(KamikazeSkies.MODID + ":orientation", Quat.class);

    /** The model is authored at 1 unit = 1/16 block with ~22 units across; a 5" quad is ~0.3 blocks wide. */
    private static final float MODEL_SCALE = 0.22f;

    public DroneRenderer(EntityRendererProvider.Context context) {
        super(context, new DefaultedEntityGeoModel<>(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "fpv_drone")));
        withScale(MODEL_SCALE);
    }

    @Override
    public void captureDefaultRenderState(DroneEntity drone, Void relatedObject, EntityRenderState state, float partialTick) {
        super.captureDefaultRenderState(drone, relatedObject, state, partialTick);
        // GeckoLib's mixin makes every EntityRenderState a GeoRenderState at runtime.
        ((GeoRenderState) state).addGeckolibData(ORIENTATION, drone.renderOrientation(partialTick));
    }

    @Override
    @SuppressWarnings("rawtypes")   // raw on purpose: EntityRenderState only satisfies RenderPassInfo's bound via GeckoLib's runtime mixin
    protected void applyRotations(RenderPassInfo info, PoseStack poseStack, float nativeScale) {
        GeoRenderState geo = (GeoRenderState) info.renderState();
        Quat q = geo.getOrDefaultGeckolibData(ORIENTATION, Quat.IDENTITY);
        poseStack.mulPose(new Matrix4f().rotation(new Quaternionf(q.x(), q.y(), q.z(), q.w())));
        // Bedrock/Blockbench models face -Z; the flight body frame's nose is +Z.
        poseStack.mulPose(new Matrix4f().rotation(Axis.YP.rotationDegrees(180f)));
    }
}
