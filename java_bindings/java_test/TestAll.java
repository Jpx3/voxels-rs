import java.io.InputStream;
import de.richy.voxels.Block;
import de.richy.voxels.Voxels;
import de.richy.voxels.BlockInputStream;
import de.richy.voxels.SchematicType;

public class TestAll {
    public static void main(String[] args) {
        try (InputStream is = TestAll.class.getResourceAsStream("sponge3.vxl")) {
            pullBlocks(is);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    static void pullBlocks(InputStream is) {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.SPONGE_V3)
        ) {
            Block[] buffer = new Block[4096];
            int read;
            while ((read = bis.read(buffer)) != -1) {
                for (int i = 0; i < read; i++) {
                    Block b = buffer[i];
                    System.out.println("Block: " + b);
                }
            }
        }
    }
}