package com.kamikazeskies.item;

import java.util.function.Consumer;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.build.PartKind;

import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.TooltipFlag;
import net.minecraft.world.item.component.TooltipDisplay;

/** A craftable drone part. Combine one of each required part in a crafting grid to assemble a drone. */
public class PartItem extends Item {
    private final PartKind kind;
    private final int tier;

    public PartItem(Properties properties, PartKind kind, int tier) {
        super(properties);
        this.kind = kind;
        this.tier = tier;
    }

    public PartKind kind() { return kind; }
    public int tier() { return tier; }

    public static @Nullable PartItem of(ItemStack stack) { return stack.getItem() instanceof PartItem p ? p : null; }

    @Override
    public void appendHoverText(ItemStack stack, Item.TooltipContext context, TooltipDisplay display, Consumer<Component> out, TooltipFlag flag) {
        if (kind.tiered) out.accept(Component.translatable("tooltip.kamikaze_skies.tier", tier).withStyle(ChatFormatting.GOLD));
        String stat = switch (kind) {
            case FRAME, MOTOR, ANTENNA, CAMERA -> String.format("%.1f", DroneBuild.partStat(kind, tier));
            case PROP, BATTERY -> String.format("%.0f", DroneBuild.partStat(kind, tier));
            case FLIGHT_CONTROLLER -> String.format("%.2f", DroneBuild.partStat(kind, tier));
            default -> "";
        };
        out.accept(Component.translatable("tooltip.kamikaze_skies.stat." + kind.id, stat).withStyle(ChatFormatting.GRAY));
        out.accept(Component.translatable("tooltip.kamikaze_skies.mass", Math.round(DroneBuild.partMass(kind, tier) * 1000)).withStyle(ChatFormatting.DARK_GRAY));
        FlightLog log = stack.get(KsComponents.FLIGHT_LOG.get());
        if (kind == PartKind.BLACKBOX && log != null) {
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.cause." + log.cause()).withStyle(ChatFormatting.RED));
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.flight", String.format("%d:%02d", (int) log.flightSeconds() / 60, (int) log.flightSeconds() % 60)).withStyle(ChatFormatting.GRAY));
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.speed", String.format("%.0f", log.maxSpeedKmh())).withStyle(ChatFormatting.GRAY));
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.altitude", String.format("%.0f", log.maxAltitude())).withStyle(ChatFormatting.GRAY));
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.volts", String.format("%.1f", log.minVolts())).withStyle(ChatFormatting.GRAY));
            out.accept(Component.translatable("tooltip.kamikaze_skies.log.signal", Math.round(log.minRssi() * 100)).withStyle(ChatFormatting.GRAY));
        }
    }
}
