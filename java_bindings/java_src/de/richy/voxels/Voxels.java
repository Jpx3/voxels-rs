package de.richy.voxels;

import java.io.InputStream;
import java.io.OutputStream;

public class Voxels {
  public static BlockInputStream blocksFromBytes(InputStream inputStream) {
    return blocksFromBytes(inputStream, SchematicType.UNKNOWN);
  }

  public static native BlockInputStream blocksFromBytes(InputStream inputStream, SchematicType schematicType);

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

  static {
    System.loadLibrary("voxels_java");
    init0();
  }
}