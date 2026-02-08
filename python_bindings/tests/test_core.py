import voxels_rs

def test_voxel_class():
  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/mojang.schem") as schematic:
    print(schematic)
    for chunk in schematic.iter_bulks():
      for block in chunk:
        print(block)

  exit(1)