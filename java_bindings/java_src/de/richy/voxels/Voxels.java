package de.richy.voxels;

import java.io.InputStream;

public class Voxels {
    public static BlockInputStream blocksFromBytes(InputStream inputStream) {
        return blocksFromBytes(inputStream, SchematicType.VXL);
    }

    public static native BlockInputStream blocksFromBytes(InputStream inputStream, SchematicType schematicType);

    public static synchronized void initialize() {
        // No-op method to ensure the class is loaded and the native library is initialized.
    }

    static {
        System.loadLibrary("voxels_java");
    }
}