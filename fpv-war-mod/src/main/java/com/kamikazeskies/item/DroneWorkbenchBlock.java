package com.kamikazeskies.item;

import com.kamikazeskies.core.KsRegistries;

import net.minecraft.core.BlockPos;
import net.minecraft.network.chat.Component;
import net.minecraft.world.MenuProvider;
import net.minecraft.world.SimpleMenuProvider;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.ContainerLevelAccess;
import net.minecraft.world.inventory.CraftingMenu;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.CraftingTableBlock;
import net.minecraft.world.level.block.state.BlockState;

/** The Drone Workbench: a crafting table of its own, where drones are assembled. */
public class DroneWorkbenchBlock extends CraftingTableBlock {
    private static final Component TITLE = Component.translatable("container.kamikaze_skies.workbench");

    public DroneWorkbenchBlock(Properties properties) { super(properties); }

    @Override
    protected MenuProvider getMenuProvider(BlockState state, Level level, BlockPos pos) {
        return new SimpleMenuProvider((id, inventory, player) -> new WorkbenchMenu(id, inventory, ContainerLevelAccess.create(level, pos)), TITLE);
    }

    /** A crafting menu that stays open as long as it is standing at a drone workbench (vanilla's checks for a crafting table). */
    public static class WorkbenchMenu extends CraftingMenu {
        private final ContainerLevelAccess access;

        public WorkbenchMenu(int id, net.minecraft.world.entity.player.Inventory inventory, ContainerLevelAccess access) {
            super(id, inventory, access);
            this.access = access;
        }

        @Override
        public boolean stillValid(Player player) { return stillValid(access, player, KsRegistries.WORKBENCH.get()); }
    }
}
