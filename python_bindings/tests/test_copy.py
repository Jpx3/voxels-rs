import voxels_rs
import time
def test_voxel_class():

  start = time.time()

  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/sponge3-monster.schem") as schematic:
    schematic.save("C:/Users/strun/RustroverProjects/voxels-rs/test_data/sponge3-monster.vxl", format="vxl")

  end = time.time()
  print(f"Transfer: {end - start:.2f} seconds")

  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/sponge3-monster.schem") as schematic:
    start = time.time()
    all_blocks = schematic.read_full()
    end = time.time()
    duration_ms = (end - start) * 1000
    print(f"Sponge read: {duration_ms:.2f} ms")

  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/sponge3-monster.vxl") as schematic:
    start = time.time()
    all_blocks = schematic.read_full()
    end = time.time()
    duration_ms = (end - start) * 1000
    print(f"VXL read: {duration_ms:.2f} ms")

  exit(1)

if __name__ == "__main__":
  test_voxel_class()