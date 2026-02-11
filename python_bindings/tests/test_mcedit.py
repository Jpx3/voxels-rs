import voxels_rs
import time

def test_voxel_class():
  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/mcedit.schematic") as schematic:
    schematic.save("C:/Users/strun/RustroverProjects/voxels-rs/test_data/mcedit.schem", format="sponge")

if __name__ == "__main__":
  test_voxel_class()