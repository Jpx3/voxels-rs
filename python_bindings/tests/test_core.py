import voxels_rs

def test_voxel_class():
  with voxels_rs.open("C:/Users/strun/RustroverProjects/voxels-rs/test_data/mojang.schem") as schematic:
#     print(schematic)
#     for chunk in schematic.iter_bulks():
#       for block in chunk:
#         print(block)
    print(type(schematic))
    import time
    start = time.time()
    all_blocks = schematic.read_full()
    end = time.time()
    duration_ms = (end - start) * 1000
    print(f"Time taken to read full schematic: {duration_ms:.2f} ms")
#     print type of all_blocks
#     print(all_blocks)

  exit(1)