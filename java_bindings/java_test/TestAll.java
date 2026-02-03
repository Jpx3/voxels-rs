import java.io.InputStream;
import java.io.OutputStream;
import de.richy.voxels.Block;
import de.richy.voxels.Voxels;
import de.richy.voxels.BlockInputStream;
import de.richy.voxels.BlockOutputStream;
import de.richy.voxels.SchematicType;
import java.io.FileInputStream;
import java.io.FileOutputStream;

public class TestAll {
    public static void main(String[] args) {
        try (
            InputStream is = new FileInputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.schem");
            OutputStream os = new FileOutputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang_out.schem");
        ) {
            pullBlocks(is, os);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    static void pullBlocks(InputStream is, OutputStream os) throws Exception {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.MOJANG);
          BlockOutputStream bos = Voxels.blocksToBytes(os, SchematicType.MOJANG);
        ) {
            long start = System.currentTimeMillis();
            Block[] buffer = new Block[512];
            int read;
            while ((read = bis.read(buffer)) != -1) {
//                 for (int i = 0; i < read; i++) {
//                     Block b = buffer[i];
//                     if (b.position().x() == 0 && b.position().y() == 0 && b.position().z() == 0) {
//                         System.out.println("Block: " + b);
//                     }
//                 }
                bos.write(buffer, 0, read);
            }
            long end = System.currentTimeMillis();
            System.out.println("Time taken: " + (end - start) + " ms");
//             System.out.println("BlockState RefCnt: " + de.richy.voxels.BlockState.ref_cnt);
//             System.out.println("BlockPosition RefCnt: " + de.richy.voxels.BlockPosition.refCnt);
        }
    }
}