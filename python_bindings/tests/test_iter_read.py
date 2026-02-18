import voxels_rs
import time

def test_voxel_class():
  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/some.vxl") as schematic:
#     schematic.iterate_blocks(lambda block: print(block))
    schematic.save("C:/Users/strun/RustroverProjects/voxels-rs/test_data/some.schem", format="mojang")

if __name__ == "__main__":
  test_voxel_class()