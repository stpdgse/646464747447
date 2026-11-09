package com.openfpv.bay;

import com.openfpv.item.ItemModule;
import com.openfpv.item.ModuleType;
import net.minecraft.inventory.InventoryBasic;
import net.minecraft.item.ItemStack;
import net.minecraft.nbt.NBTTagCompound;
import net.minecraft.nbt.NBTTagList;

/** Reads/writes the module bay stored inside a Camera Upgrade's item NBT. */
public final class ModuleBay {
    public static final String TAG = "openfpv";
    public static final String LIST = "Modules";
    public static final int SIZE = ModuleType.VALUES.length;

    private ModuleBay() {}

    public static ItemStack[] read(ItemStack camera) {
        ItemStack[] out = new ItemStack[SIZE];
        for (int i = 0; i < SIZE; i++) out[i] = ItemStack.EMPTY;
        if (camera.isEmpty() || !camera.hasTagCompound()) return out;
        NBTTagCompound root = camera.getTagCompound();
        if (root == null || !root.hasKey(TAG)) return out;
        NBTTagList list = root.getCompoundTag(TAG).getTagList(LIST, 10);
        for (int i = 0; i < list.tagCount(); i++) {
            NBTTagCompound t = list.getCompoundTagAt(i);
            int slot = t.getByte("Slot") & 0xFF;
            if (slot < SIZE) out[slot] = new ItemStack(t);
        }
        return out;
    }

    public static void write(ItemStack camera, ItemStack[] modules) {
        NBTTagList list = new NBTTagList();
        for (int i = 0; i < SIZE; i++) {
            if (!modules[i].isEmpty()) {
                NBTTagCompound t = new NBTTagCompound();
                modules[i].writeToNBT(t);
                t.setByte("Slot", (byte) i);
                list.appendTag(t);
            }
        }
        NBTTagCompound root = camera.hasTagCompound() ? camera.getTagCompound() : new NBTTagCompound();
        NBTTagCompound mine = new NBTTagCompound();
        mine.setTag(LIST, list);
        root.setTag(TAG, mine);
        camera.setTagCompound(root);
    }

    /** Bit mask of installed modules (see {@link ModuleType#bit()}). */
    public static int readMask(ItemStack camera) {
        int mask = 0;
        for (ItemStack s : read(camera)) {
            if (!s.isEmpty() && s.getItem() instanceof ItemModule) {
                mask |= ((ItemModule) s.getItem()).type.bit();
            }
        }
        return mask;
    }

    /** Inventory view of the bay; every change is written straight back into the camera stack. */
    public static class Inv extends InventoryBasic {
        private final ItemStack camera;
        private boolean loading;

        public Inv(ItemStack camera) {
            super("container.openfpv.module_bay", false, SIZE);
            this.camera = camera;
            loading = true;
            ItemStack[] s = read(camera);
            for (int i = 0; i < SIZE; i++) setInventorySlotContents(i, s[i]);
            loading = false;
        }

        @Override
        public int getInventoryStackLimit() {
            return 1;
        }

        @Override
        public boolean isItemValidForSlot(int index, ItemStack stack) {
            return stack.getItem() instanceof ItemModule && ((ItemModule) stack.getItem()).type.ordinal() == index;
        }

        @Override
        public void markDirty() {
            super.markDirty();
            if (loading) return;
            ItemStack[] s = new ItemStack[SIZE];
            for (int i = 0; i < SIZE; i++) s[i] = getStackInSlot(i);
            write(camera, s);
        }
    }
}
