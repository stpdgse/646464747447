package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.ModSounds;
import com.openfpv.OpenFPV;
import java.awt.image.BufferedImage;
import java.io.File;
import java.io.IOException;
import java.io.OutputStream;
import java.io.PrintWriter;
import java.nio.ByteBuffer;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;
import javax.imageio.ImageIO;
import net.minecraft.client.Minecraft;
import net.minecraft.client.shader.Framebuffer;
import net.minecraft.util.text.TextComponentString;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;
import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;

/**
 * DVR: records the FPV feed (with the full OSD) at 20 frames per second, one frame per game tick, and writes a
 * "blackbox" CSV of the flight telemetry next to it. Frames are streamed to ffmpeg (mp4 / H.264) when it is
 * installed; otherwise they are saved as a PNG sequence. Files go to <game dir>/openfpv/recordings/.
 */
@SideOnly(Side.CLIENT)
public final class Recorder {
    private Recorder() {}

    private static final byte[] POISON = new byte[0];

    private static volatile boolean recording;
    private static volatile boolean finished = true;
    private static BlockingQueue<byte[]> queue;
    private static Thread writer;
    private static ByteBuffer buffer;
    private static int width, height;
    private static long lastTick = Long.MIN_VALUE;
    private static long startTick;
    private static int frames, dropped;
    private static PrintWriter csv;
    private static File videoFile, csvFile, framesDir;
    private static boolean usingFfmpeg;

    public static boolean isRecording() {
        return recording;
    }

    /** True once the last recording has been completely written out. */
    public static boolean isFinished() {
        return finished;
    }

    public static int frameCount() {
        return frames;
    }

    public static File video() {
        return videoFile;
    }

    public static File blackbox() {
        return csvFile;
    }

    public static File framesFolder() {
        return framesDir;
    }

    public static int seconds() {
        Minecraft mc = Minecraft.getMinecraft();
        return mc.world == null ? 0 : (int) ((mc.world.getTotalWorldTime() - startTick) / 20);
    }

    public static void toggle() {
        if (recording) stop(); else start();
    }

    public static synchronized void start() {
        Minecraft mc = Minecraft.getMinecraft();
        if (recording || mc.world == null) return;
        Framebuffer fb = mc.getFramebuffer();
        width = fb.framebufferWidth;
        height = fb.framebufferHeight;
        buffer = BufferUtils.createByteBuffer(width * height * 3);
        File dir = new File(mc.mcDataDir, "openfpv/recordings");
        dir.mkdirs();
        String stamp = new SimpleDateFormat("yyyyMMdd_HHmmss").format(new Date());
        videoFile = new File(dir, "flight_" + stamp + ".mp4");
        csvFile = new File(dir, "flight_" + stamp + ".csv");
        framesDir = null;
        frames = 0;
        dropped = 0;
        lastTick = Long.MIN_VALUE;
        startTick = mc.world.getTotalWorldTime();
        queue = new ArrayBlockingQueue<>(12);

        Process ffmpeg = null;
        try {
            ffmpeg = new ProcessBuilder(Config.ffmpegPath, "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
                "-s", width + "x" + height, "-r", "20", "-i", "-",
                "-vf", "vflip,scale=trunc(iw/2)*2:trunc(ih/2)*2", "-c:v", "libx264", "-preset", "veryfast", "-crf", "20",
                "-pix_fmt", "yuv420p", videoFile.getAbsolutePath()).redirectErrorStream(true).start();
            usingFfmpeg = true;
        } catch (IOException ex) {
            usingFfmpeg = false;
            framesDir = new File(dir, "flight_" + stamp + "_frames");
            framesDir.mkdirs();
            OpenFPV.log.info("ffmpeg not found ('{}'): recording a PNG sequence to {}", Config.ffmpegPath, framesDir);
        }
        try {
            csv = new PrintWriter(csvFile, "UTF-8");
            csv.println("tick,x,y,z,yaw,pitch,roll,speed_ms,battery_pct,signal_pct,distance_m,lag_ms,mode");
        } catch (IOException ex) {
            csv = null;
        }
        final Process proc = ffmpeg;
        final BlockingQueue<byte[]> q = queue;
        final int w = width, h = height;
        finished = false;
        recording = true;
        writer = new Thread(() -> write(q, proc, w, h), "OpenFPV-DVR");
        writer.setDaemon(true);
        writer.start();
        FpvClientHooks.recStarted();
    }

    private static void write(BlockingQueue<byte[]> q, Process proc, int w, int h) {
        int n = 0;
        try (OutputStream out = proc == null ? null : proc.getOutputStream()) {
            while (true) {
                byte[] f = q.take();
                if (f == POISON) break;
                if (out != null) {
                    out.write(f);
                } else if (framesDir != null) {
                    BufferedImage img = new BufferedImage(w, h, BufferedImage.TYPE_INT_RGB);
                    for (int y = 0; y < h; y++) {
                        int row = (h - 1 - y) * w * 3;
                        for (int x = 0; x < w; x++) {
                            int i = row + x * 3;
                            img.setRGB(x, y, ((f[i] & 255) << 16) | ((f[i + 1] & 255) << 8) | (f[i + 2] & 255));
                        }
                    }
                    ImageIO.write(img, "png", new File(framesDir, String.format("frame_%05d.png", n++)));
                }
            }
        } catch (Exception ex) {
            OpenFPV.log.warn("DVR writer stopped: {}", ex.toString());
        }
        if (proc != null) {
            try {
                proc.waitFor();
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
            }
        }
        finished = true;
    }

    /** Called at the end of every rendered frame; grabs at most one frame per game tick. */
    public static void frame() {
        Minecraft mc = Minecraft.getMinecraft();
        if (!recording || mc.world == null) return;
        long tick = mc.world.getTotalWorldTime();
        if (tick == lastTick) return;
        lastTick = tick;
        buffer.clear();
        GL11.glPixelStorei(GL11.GL_PACK_ALIGNMENT, 1);
        GL11.glReadPixels(0, 0, width, height, GL11.GL_RGB, GL11.GL_UNSIGNED_BYTE, buffer);
        byte[] f = new byte[width * height * 3];
        buffer.get(f);
        if (queue.offer(f)) frames++; else dropped++;     // never stall the game for the encoder
        telemetry(mc, tick);
    }

    private static void telemetry(Minecraft mc, long tick) {
        if (csv == null || mc.getRenderViewEntity() == null) return;
        FpvClient.Cam cam = FpvClient.current();
        float[] att = FpvClient.attitude();
        net.minecraft.entity.Entity e = mc.getRenderViewEntity();
        csv.println(String.format(java.util.Locale.ROOT, "%d,%.3f,%.3f,%.3f,%.1f,%.1f,%.1f,%.2f,%.0f,%.0f,%.1f,%d,%s",
            tick - startTick, e.posX, e.posY, e.posZ, att[0], att[1], att[2], FpvClient.speed,
            cam == null ? 0 : cam.energy * 100, FpvClient.quality * 100, FpvClient.distance, FpvClient.lagTicks * 50,
            FpvClient.isAcro() ? "ACRO" : "ANGLE"));
    }

    public static synchronized void stop() {
        if (!recording) return;
        recording = false;
        if (csv != null) {
            csv.close();
            csv = null;
        }
        try {
            queue.put(POISON);   // the writer finishes the file in the background
        } catch (InterruptedException ex) {
            Thread.currentThread().interrupt();
        }
        FpvClientHooks.recStopped(usingFfmpeg ? videoFile : framesDir, csvFile, frames, dropped);
    }

    static void message(String text) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.player != null) mc.player.sendMessage(new TextComponentString(text));
    }

    /** Small indirection so the recorder stays free of FpvClient's static state during start/stop. */
    static final class FpvClientHooks {
        static void recStarted() {
            DroneSounds.play(ModSounds.REC_CLICK);
        }

        static void recStopped(File target, File log, int frames, int dropped) {
            DroneSounds.play(ModSounds.REC_CLICK);
            message("§aOpenFPV: saved " + (target == null ? "?" : target.getName()) + " (" + frames + " frames"
                + (dropped > 0 ? ", " + dropped + " dropped" : "") + ") and " + log.getName());
        }
    }
}
