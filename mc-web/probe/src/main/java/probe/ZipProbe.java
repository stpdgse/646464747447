package probe;

import java.nio.file.FileSystem;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.spi.FileSystemProvider;
import java.util.stream.Collectors;

public class ZipProbe {
    static byte[] bytes() {
        String h = "504b0304140000000800573c775d8ceea483150000006f000000200000006173736574732f6d696e6563726166742f6c616e672f656e5f75732e6a736f6eab564a54b252ca48cdc9c957c0412ad556534d1100504b0304140000000800573c775d0372e863100000000e000000250000006173736574732f6d696e6563726166742f736861646572732f636f72652f6775692e7673682bcbcf4c51c84dccccd3d0acaee50200504b0304140000000800573c775df8478294180000001a0000000b0000007061636b2e6d636d657461ab562a484cce56b2aa06d3f169f945b989254a5686b5b500504b03041400000000000000210000000000000000000000000005000000646174612f504b0304140000000800573c775d56bc29470c0000000a00000014000000646174612f6d696e6563726166742f782e7478742b2ec92f4a4dd1cd2cce0000504b01021403140000000800573c775d8ceea483150000006f0000002000000000000000000000008001000000006173736574732f6d696e6563726166742f6c616e672f656e5f75732e6a736f6e504b01021403140000000800573c775d0372e863100000000e0000002500000000000000000000008001530000006173736574732f6d696e6563726166742f736861646572732f636f72652f6775692e767368504b01021403140000000800573c775df8478294180000001a0000000b00000000000000000000008001a60000007061636b2e6d636d657461504b01021403140000000000000021000000000000000000000000000500000000000000000000008001e7000000646174612f504b01021403140000000800573c775d56bc29470c0000000a00000014000000000000000000000080010a010000646174612f6d696e6563726166742f782e747874504b050600000000050005004f010000480100000000";
        byte[] b = new byte[h.length() / 2];
        for (int i = 0; i < b.length; i++) {
            b[i] = (byte) Integer.parseInt(h.substring(i * 2, i * 2 + 2), 16);
        }
        return b;
    }

    public static void main(String[] args) throws Exception {
        FileSystemProvider jar = null;
        for (FileSystemProvider p : FileSystemProvider.installedProviders()) {
            System.out.println("provider: " + p.getScheme());
            if (p.getScheme().equals("jar")) {
                jar = p;
            }
        }
        FileSystem fs = ((org.teavm.mcweb.fs.ZipFileSystemProvider) jar).newFileSystem(bytes());
        Path root = fs.getPath("/");
        System.out.println("isDir root: " + Files.isDirectory(root));
        Path lang = fs.getPath("assets/minecraft/lang/en_us.json");
        System.out.println("exists: " + Files.exists(lang) + " size: " + Files.size(lang));
        System.out.println("content: " + new String(Files.readAllBytes(lang)).substring(0, 20));
        System.out.println("walk: " + Files.walk(root).map(Path::toString).sorted().collect(Collectors.toList()));
        System.out.println("list: " + Files.list(fs.getPath("/assets/minecraft")).map(Path::toString).collect(Collectors.toList()));
        System.out.println("resolve: " + root.resolve("data").resolve("minecraft/x.txt") + " "
                + new String(Files.readAllBytes(root.resolve("data/minecraft/x.txt"))));
        System.out.println("missing: " + Files.exists(fs.getPath("/nope")));
    }
}
