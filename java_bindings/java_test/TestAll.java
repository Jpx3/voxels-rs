import java.io.*;
import java.util.*;
import de.richy.voxels.*;

public class TestAll {
  private static final String BASE_PATH = "C:/Users/strun/RustroverProjects/voxels-rs/test_data/generation_test/";

  public static void main(String[] args) throws IOException {
    writeTreeSchematic();
    convert("tree.schematic", SchematicType.MOJANG, "tree.vxl", SchematicType.VXL);
    convert("tree.vxl", SchematicType.VXL, "tree.schem", SchematicType.SPONGE);
    convert("tree.schem", SchematicType.SPONGE, "tree_mojang.schematic", SchematicType.MOJANG);
  }

  private static void writeTreeSchematic() throws IOException {
    Block[] treeBlocks = setupTestingSchematic();
    File outFile = new File(BASE_PATH, "tree.schematic");

    try (OutputStream os = new FileOutputStream(outFile);
         BlockOutputStream bos = Voxels.blocksToBytes(os, SchematicType.MOJANG)) {
      bos.write(treeBlocks, 0, treeBlocks.length);
    }
  }

  private static void convert(String inName, SchematicType inType, String outName, SchematicType outType) throws IOException {
    File inFile = new File(BASE_PATH, inName);
    File outFile = new File(BASE_PATH, outName);

    try (InputStream is = new FileInputStream(inFile);
         BlockInputStream bis = Voxels.bytesToBlocks(is, inType);
         OutputStream os = new FileOutputStream(outFile);
         BlockOutputStream bos = Voxels.blocksToBytes(os, outType, bis.boundary())) {
      pipeBlocks(bis, bos);
    }
  }

  private static void pipeBlocks(BlockInputStream bis, BlockOutputStream bos) throws IOException {
    Block[] buffer = new Block[512];
    int read;
    while ((read = bis.read(buffer, 0, buffer.length)) != -1) {
      bos.write(buffer, 0, read);
    }
  }

  private static Block[] setupTestingSchematic() {
    int width = 16, height = 16, length = 16;
    Block[] blocks = new Block[width * height * length];

    int trunkX = 8, trunkZ = 8, trunkHeight = 5, leafStart = 3;

    for (int x = 0; x < width; x++) {
      for (int y = 0; y < height; y++) {
        for (int z = 0; z < length; z++) {
          int index = x + (y * 16) + (z * 16 * 16);
          int dx = Math.abs(x - trunkX);
          int dz = Math.abs(z - trunkZ);
          int distSq = dx * dx + dz * dz;

          String type = "minecraft:air";
          Map<String, String> props = new HashMap<>();

          if (dx == 0 && dz == 0 && y < trunkHeight) {
            type = "minecraft:oak_log";
            props.put("axis", new String[]{"y", "x", "z"}[y % 3]);
          } else if (y >= leafStart && y <= trunkHeight + 1) {
            int radius = (y == trunkHeight + 1) ? 2 : 3;
            if (distSq < radius * radius && !(dx == radius - 1 && dz == radius - 1)) {
              type = "minecraft:oak_leaves";
              props.put("distance", "1");
              props.put("persistent", "true");
            }
          }

          blocks[index] = new Block(BlockPosition.of(x, y, z), BlockState.of(type, props));
        }
      }
    }
    return blocks;
  }
}