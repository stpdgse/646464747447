package org.teavm.mcweb;

import org.teavm.vm.spi.TeaVMHost;
import org.teavm.vm.spi.TeaVMPlugin;

/** Registers the mc-web build-time transformations. */
public class McWebPlugin implements TeaVMPlugin {
    @Override
    public void install(TeaVMHost host) {
        host.add(new JdkGapTransformer());
    }
}
