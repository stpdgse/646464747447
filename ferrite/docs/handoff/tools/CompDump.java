import net.minecraft.SharedConstants;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.component.DataComponentType;
import net.minecraft.server.Bootstrap;

public class CompDump {
    public static void main(String[] a) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        for (DataComponentType<?> t : BuiltInRegistries.DATA_COMPONENT_TYPE) {
            var id = BuiltInRegistries.DATA_COMPONENT_TYPE.getKey(t);
            String c;
            try { c = String.valueOf(t.codec()); } catch (Throwable e) { c = "ERR " + e; }
            System.out.println(id + " :: " + c);
        }
    }
}
