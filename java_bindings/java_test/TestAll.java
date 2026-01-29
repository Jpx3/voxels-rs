import java.io.InputStream;
import de.richy.voxels.Block;
import de.richy.voxels.Voxels;
import de.richy.voxels.BlockInputStream;
import de.richy.voxels.SchematicType;
import java.io.FileInputStream;

public class TestAll {
    public static void main(String[] args) {
        try (InputStream is = new FileInputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.schem")) {
            pullBlocks(is);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    static void pullBlocks(InputStream is) {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.MOJANG)
        ) {
            long start = System.currentTimeMillis();
            Block[] buffer = new Block[512];
            int read;
            while ((read = bis.read(buffer)) != -1) {
                for (int i = 0; i < read; i++) {
                    Block b = buffer[i];
//                     System.out.println("Block: " + b);
                }
            }


            long end = System.currentTimeMillis();
            System.out.println("Time taken: " + (end - start) + " ms");
            System.out.println("BlockState RefCnt: " + de.richy.voxels.BlockState.refCnt);
            System.out.println("BlockPosition RefCnt: " + de.richy.voxels.BlockPosition.refCnt);
        }
    }
}