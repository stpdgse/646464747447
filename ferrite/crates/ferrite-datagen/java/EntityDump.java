import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.ai.attributes.Attribute;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.attributes.DefaultAttributes;

/**
 * Dumps what the simulation needs to know about every entity type that the data generator's
 * reports leave out: hitbox size, mob category, tracking range and default attributes.
 *
 * One line per type, in registry order:
 * {@code name width height eye category tracking interval fireImmune summonable living
 *        max_health movement_speed attack_damage follow_range knockback_resistance armor}
 * Attributes a type does not have are written as {@code -}.
 */
public class EntityDump {
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Bootstrap.validate();
        List<String> rows = new ArrayList<>();
        for (EntityType<?> type : BuiltInRegistries.ENTITY_TYPE) {
            String name = BuiltInRegistries.ENTITY_TYPE.getKey(type).toString();
            var dims = type.getDimensions();
            StringBuilder sb = new StringBuilder();
            sb.append(name).append(' ')
                .append(dims.width()).append(' ').append(dims.height()).append(' ').append(dims.eyeHeight()).append(' ')
                .append(type.getCategory().getName()).append(' ')
                .append(type.clientTrackingRange()).append(' ').append(type.updateInterval()).append(' ')
                .append(type.fireImmune() ? 1 : 0).append(' ').append(type.canSummon() ? 1 : 0).append(' ');
            boolean living = DefaultAttributes.hasSupplier(type);
            sb.append(living ? 1 : 0);
            var attrs = new net.minecraft.core.Holder[] {
                Attributes.MAX_HEALTH, Attributes.MOVEMENT_SPEED, Attributes.ATTACK_DAMAGE,
                Attributes.FOLLOW_RANGE, Attributes.KNOCKBACK_RESISTANCE, Attributes.ARMOR };
            for (var a : attrs) {
                sb.append(' ');
                if (living) {
                    AttributeSupplier sup = DefaultAttributes.getSupplier((EntityType<? extends LivingEntity>) type);
                    if (sup.hasAttribute(a)) { sb.append(sup.getValue(a)); } else { sb.append('-'); }
                } else {
                    sb.append('-');
                }
            }
            rows.add(sb.toString());
        }
        try (PrintWriter w = new PrintWriter(args[0], StandardCharsets.UTF_8)) {
            w.println("entities " + rows.size());
            for (String r : rows) w.println(r);
        }
        System.out.println("entities=" + rows.size());
    }
}
