package org.teavm.mcweb;

/**
 * Browser entry point. Starts the game the way the official launcher would, with fixed offline
 * arguments. The page can later replace these through a JavaScript-provided argument list.
 */
public final class McWebMain {
    private McWebMain() {
    }

    public static void main(String[] args) throws Exception {
        byte[] archive = org.teavm.mcweb.host.WebHost.archive();
        if (archive != null) {
            McWebResources.install(archive);
            System.out.println("[mcweb] resource archive installed: " + archive.length + " bytes");
        } else {
            System.out.println("[mcweb] no resource archive supplied by the page");
        }
        McWebResources.ensureDirectory(java.nio.file.Paths.get("/game"));
        String[] launch = {
            "--version", "26.3",
            "--username", "Player",
            "--uuid", "00000000-0000-0000-0000-000000000000",
            "--accessToken", "0",
            "--userType", "legacy",
            "--gameDir", "/game",
            "--assetsDir", "/launcher-assets",
            "--assetIndex", "34",
            "--width", "854",
            "--height", "480",
            "--offlineDeveloperMode",
            "--graphicsBackend", "opengl",
        };
        net.minecraft.client.main.Main.main(launch);
    }
}
