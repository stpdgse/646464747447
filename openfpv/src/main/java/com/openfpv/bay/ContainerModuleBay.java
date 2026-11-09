package com.openfpv.bay;

import com.openfpv.ModItems;
import com.openfpv.item.ItemAntenna;
import com.openfpv.item.ItemModule;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.entity.player.InventoryPlayer;
import net.minecraft.inventory.ClickType;
import net.minecraft.inventory.Container;
import net.minecraft.inventory.IInventory;
import net.minecraft.inventory.Slot;
import net.minecraft.item.ItemStack;
import net.minecraft.util.EnumHand;

public class ContainerModuleBay extends Container {
    public static final int BAY_X = 34;
    public static final int BAY_Y = 20;

    private final ItemStack camera;
    private final EnumHand hand;
    private final EntityPlayer player;
    private final ModuleBay.Inv bay;

    public ContainerModuleBay(InventoryPlayer inv, EnumHand hand) {
        this.player = inv.player;
        this.hand = hand;
        this.camera = inv.player.getHeldItem(hand);
        this.bay = new ModuleBay.Inv(camera);

        for (int i = 0; i < ModuleBay.SIZE; i++) {
            addSlotToContainer(new Slot(bay, i, BAY_X + i * 18, BAY_Y) {
                @Override
                public boolean isItemValid(ItemStack stack) {
                    return bay.isItemValidForSlot(getSlotIndex(), stack);
                }

                @Override
                public int getSlotStackLimit() {
                    return 1;
                }
            });
        }
        for (int row = 0; row < 3; row++) {
            for (int col = 0; col < 9; col++) {
                addSlotToContainer(new Slot(inv, col + row * 9 + 9, 8 + col * 18, 51 + row * 18));
            }
        }
        for (int col = 0; col < 9; col++) {
            addSlotToContainer(new Slot(inv, col, 8 + col * 18, 109));
        }
    }

    public ItemStack getCamera() {
        return camera;
    }

    public IInventory getBay() {
        return bay;
    }

    @Override
    public boolean canInteractWith(EntityPlayer p) {
        ItemStack held = p.getHeldItem(hand);
        return held == camera && !held.isEmpty() && held.getItem() == ModItems.CAMERA;
    }

    @Override
    public ItemStack slotClick(int slotId, int dragType, ClickType clickType, EntityPlayer p) {
        // The camera being edited must not be moved around while its bay is open.
        if (slotId >= 0 && slotId < inventorySlots.size() && inventorySlots.get(slotId).getStack() == camera) {
            return ItemStack.EMPTY;
        }
        if (clickType == ClickType.SWAP && dragType >= 0 && dragType < 9
            && p.inventory.getStackInSlot(dragType) == camera) {
            return ItemStack.EMPTY;
        }
        return super.slotClick(slotId, dragType, clickType, p);
    }

    @Override
    public ItemStack transferStackInSlot(EntityPlayer p, int index) {
        Slot slot = inventorySlots.get(index);
        if (slot == null || !slot.getHasStack() || slot.getStack() == camera) return ItemStack.EMPTY;
        ItemStack stack = slot.getStack();
        ItemStack copy = stack.copy();
        if (index < ModuleBay.SIZE) {
            if (!mergeItemStack(stack, ModuleBay.SIZE, inventorySlots.size(), true)) return ItemStack.EMPTY;
        } else if (stack.getItem() instanceof ItemModule || stack.getItem() instanceof ItemAntenna) {
            int target = stack.getItem() instanceof ItemAntenna ? ModuleBay.ANTENNA_SLOT
                : ((ItemModule) stack.getItem()).type.ordinal();
            if (inventorySlots.get(target).getHasStack()) return ItemStack.EMPTY;
            inventorySlots.get(target).putStack(stack.splitStack(1));
        } else {
            return ItemStack.EMPTY;
        }
        if (stack.isEmpty()) slot.putStack(ItemStack.EMPTY); else slot.onSlotChanged();
        return copy;
    }

    @Override
    public void onContainerClosed(EntityPlayer p) {
        super.onContainerClosed(p);
        bay.markDirty();
    }
}
