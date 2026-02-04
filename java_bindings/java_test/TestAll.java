import java.io.InputStream;
import java.io.OutputStream;
import de.richy.voxels.Block;
import de.richy.voxels.Voxels;
import de.richy.voxels.Boundary;
import de.richy.voxels.BlockInputStream;
import de.richy.voxels.BlockOutputStream;
import de.richy.voxels.SchematicType;
import java.io.FileInputStream;
import java.io.FileOutputStream;

public class TestAll {
    public static void main(String[] args) {
//         try (
//             InputStream is = new FileInputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.schem");
//             OutputStream os = new FileOutputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.vxl");
//         ) {
//             moveMojangToVXL(is, os);
//         } catch (Exception e) {
//             e.printStackTrace();
//         }
        try (
            InputStream is = new FileInputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\sponge3.schem");
            OutputStream os = new FileOutputStream("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\sponge3.vxl");
        ) {
//             readSponge(is);
            moveSpongeToVXL(is, os);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    static void moveMojangToVXL(InputStream is, OutputStream os) throws Exception {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.MOJANG);
        ) {
          Boundary boundary = bis.boundary();
          try(
            BlockOutputStream bos = Voxels.blocksToBytes(os, SchematicType.VXL, boundary);
          ) {
            long start = System.currentTimeMillis();
            Block[] buffer = new Block[512];
            int read;
            while ((read = bis.read(buffer)) != -1) {
//               for (int i = 0; i < read; i++) {
//                 Block b = buffer[i];
//                 System.out.printf("JAVA Block at (%d, %d, %d): id=%s, data=%s%n", b.position().x(), b.position().y(), b.position().z(), b.state().typeName(), b.state().properties());
//               }
              bos.write(buffer, 0, read);
            }
            long end = System.currentTimeMillis();
            System.out.println("Time taken: " + (end - start) + " ms");
          }
//             System.out.println("BlockState RefCnt: " + de.richy.voxels.BlockState.ref_cnt);
//             System.out.println("BlockPosition RefCnt: " + de.richy.voxels.BlockPosition.refCnt);
        }
    }


    static void readSponge(InputStream is) throws Exception {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.SPONGE);
        ) {
          Boundary boundary = bis.boundary();
          long start = System.currentTimeMillis();
          Block[] buffer = new Block[512];
          int read;
          int count = 0;
          while ((read = bis.read(buffer)) != -1) {
            for (int i = 0; i < read; i++) {
              Block b = buffer[i];
//               System.out.printf("JAVA Block at (%d, %d, %d): id=%s, data=%s%n", b.position().x(), b.position().y(), b.position().z(), b.state().typeName(), b.state().properties());
            count++;
            }
          }
          long end = System.currentTimeMillis();
          System.out.println("Time taken: " + (end - start) + " ms for " + count + " blocks");

        }
    }

    static void moveSpongeToVXL(InputStream is, OutputStream os) throws Exception {
        try(
          BlockInputStream bis = Voxels.blocksFromBytes(is, SchematicType.SPONGE);
        ) {
          Boundary boundary = bis.boundary();
          try(
            BlockOutputStream bos = Voxels.blocksToBytes(os, SchematicType.VXL, boundary);
          ) {
            long cnt = 0;
            long durationReader = 0;
            long durationWriter = 0;
            Block[] buffer = new Block[512];
            int read;
            while (true) {
                long startRead = System.currentTimeMillis();
                read = bis.read(buffer);
                long endRead = System.currentTimeMillis();
                durationReader += (endRead - startRead);
                if (read == -1) {
                    break;
                }
                long startWrite = System.currentTimeMillis();
                bos.write(buffer, 0, read);
                long endWrite = System.currentTimeMillis();
                durationWriter += (endWrite - startWrite);
                cnt += read;
            }
            System.out.println("Total blocks processed: " + cnt);
            System.out.println("Total reading time: " + durationReader + " ms");
            System.out.println("Total writing time: " + durationWriter + " ms");
            System.out.println("Overall time: " + (durationReader + durationWriter) + " ms");

            }
        }
    }
}