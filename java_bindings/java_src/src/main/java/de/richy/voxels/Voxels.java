package de.richy.voxels;

import java.io.InputStream;
import java.io.OutputStream;
import java.io.IOException;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

public class Voxels {
  public static BlockInputStream bytesToBlocks(InputStream inputStream) {
    return blocksFromBytes(inputStream, SchematicType.VXL);
  }

  public static BlockInputStream bytesToBlocks(InputStream inputStream, SchematicType schematicType) {
    return blocksFromBytes(inputStream, schematicType);
  }

  public static BlockInputStream blocksFromBytes(InputStream inputStream) {
    return blocksFromBytes(inputStream, SchematicType.UNKNOWN);
  }

  public static native BlockInputStream blocksFromBytes(InputStream inputStream, SchematicType schematicType);

  public static BlockOutputStream bytesFromBlocks(OutputStream outputStream) {
    return blocksToBytes(outputStream, SchematicType.VXL);
  }

  public static BlockOutputStream bytesFromBlocks(OutputStream outputStream, SchematicType schematicType) {
    return blocksToBytes(outputStream, schematicType);
  }

  public static BlockOutputStream blocksToBytes(OutputStream outputStream, Boundary boundary) {
    return blocksToBytes(outputStream, SchematicType.VXL);
  }
  
  public static BlockOutputStream blocksToBytes(OutputStream outputStream, SchematicType schematicType) {
    if (schematicType.writerRequiresBoundary()) {
      throw new IllegalArgumentException("SchematicType " + schematicType + " requires a Boundary to write.");
    }
    return blocksToBytes(outputStream, schematicType, null);
  }

  public static native BlockOutputStream blocksToBytes(OutputStream outputStream, SchematicType schematicType, Boundary boundary);

  public static synchronized void initialize() {
    // No-op: just to ensure the static block is executed.
  }

  private static native void init0();

  private static synchronized void load() {
    try {
      String osName = System.getProperty("os.name").toLowerCase();
      String osFolder = "";
      String ext = "";
      if (osName.contains("win")) {
        osFolder = "windows";
        ext = ".dll";
      } else if (osName.contains("mac")) {
        osFolder = "macos";
        ext = ".dylib";
      } else if (osName.contains("nix") || osName.contains("nux")) {
        osFolder = "linux";
        ext = ".so";
      } else {
        throw new UnsupportedOperationException("Unsupported OS: " + osName);
      }

      String arch = System.getProperty("os.arch").toLowerCase();
      if (arch.equals("amd64")) arch = "x86_64";

      String resourcePath = "/native/" + osFolder + "/" + arch + "/libvoxels_java" + ext;

      InputStream in = Voxels.class.getResourceAsStream(resourcePath);
      if (in == null) {
        File localDevFile = new File("../../target/release/libvoxels_java" + ext);
        if (localDevFile.exists()) {
            System.load(localDevFile.getAbsolutePath());
            return;
        }
        throw new FileNotFoundException("Native lib not found in JAR at: " + resourcePath);
      }

      File temp = File.createTempFile("libvoxels_java", ext);
      temp.deleteOnExit();
      Files.copy(in, temp.toPath(), StandardCopyOption.REPLACE_EXISTING);
      System.load(temp.getAbsolutePath());
    } catch (IOException e) {
      throw new RuntimeException("Failed to load native library", e);
    }
  }

  static {
    load();
    init0();
  }
}