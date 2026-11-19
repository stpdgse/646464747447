package org.teavm.mcweb;

/**
 * Browser entry point. Starts the game the way the official launcher would, with fixed offline
 * arguments. The page can later replace these through a JavaScript-provided argument list.
 */
public final class McWebMain {
    private McWebMain() {
    }

    public static void main(String[] args) {
        String[] launch = {
            "--version", "26.3",
            "--username", "Player",
            "--uuid", "00000000-0000-0000-0000-000000000000",
            "--accessToken", "0",
            "--userType", "legacy",
            "--gameDir", "/game",
            "--assetsDir", "/assets",
            "--assetIndex", "34",
            "--width", "854",
            "--height", "480",
            "--offlineDeveloperMode",
            "--graphicsBackend", "opengl",
        };
        net.minecraft.client.main.Main.main(launch);
    }
}
