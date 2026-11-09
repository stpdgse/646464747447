package com.openfpv.oc;

import com.openfpv.ModItems;
import li.cil.oc.api.driver.item.HostAware;
import li.cil.oc.api.driver.item.Slot;
import li.cil.oc.api.internal.Drone;
import li.cil.oc.api.network.EnvironmentHost;
import li.cil.oc.api.network.ManagedEnvironment;
import li.cil.oc.api.prefab.DriverItem;
import net.minecraft.item.ItemStack;

/** Makes the Camera Upgrade installable in a drone's upgrade slot. */
public class DriverCamera extends DriverItem implements HostAware {
    public DriverCamera() {
        super(new ItemStack(ModItems.CAMERA));
    }

    @Override
    public boolean worksWith(ItemStack stack, Class<? extends EnvironmentHost> host) {
        return worksWith(stack) && Drone.class.isAssignableFrom(host);
    }

    @Override
    public ManagedEnvironment createEnvironment(ItemStack stack, EnvironmentHost host) {
        if (host instanceof Drone && host instanceof net.minecraft.entity.Entity) {
            return new CameraEnvironment((Drone) host, stack);
        }
        return null;
    }

    @Override
    public String slot(ItemStack stack) {
        return Slot.Upgrade;
    }

    @Override
    public int tier(ItemStack stack) {
        return 0;
    }
}
