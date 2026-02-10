package de.richy.voxels;

import java.io.InputStream;
import java.io.OutputStream;
import java.io.IOException;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.io.FileNotFoundException;
import java.net.URL;

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
      String os = System.getProperty("os.name").toLowerCase();
      String libName;

      if (os.contains("win")) {
        libName = "voxels_java.dll";
      } else if (os.contains("mac")) {
        libName = "libvoxels_java.dylib";
      } else if (os.contains("nix") || os.contains("nux")) {
        libName = "libvoxels_java.so";
      } else {
        throw new UnsupportedOperationException("Unsupported OS: " + os);
      }

      // Attempt to load from classpath/resources
      URL resource = Voxels.class.getResource("/native/" + libName);

      if (resource == null) {
        // Fallback for local development
        File devFile = new File("target/release/" + libName);
        if (devFile.exists()) {
          System.load(devFile.getAbsolutePath());
          return;
        }
        throw new FileNotFoundException("Native library " + libName + " not found.");
      }

      // Extract to temporary file for loading
      String[] parts = libName.split("\\.");
      File temp = File.createTempFile(parts[0], "." + parts[1]);
      temp.deleteOnExit();

      try (InputStream in = resource.openStream()) {
        Files.copy(in, temp.toPath(), StandardCopyOption.REPLACE_EXISTING);
      }

      System.load(temp.getAbsolutePath());
    } catch (IOException e) {
      throw new RuntimeException("Critical failure loading native library", e);
    }
  }

  static {
    load();
    init0();
  }
}