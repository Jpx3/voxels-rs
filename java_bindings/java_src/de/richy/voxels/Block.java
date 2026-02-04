package de.richy.voxels;

public record Block(BlockPosition position, BlockState state) {
  public static Block[] filledBuffer(int size) {
    return filledBuffer(size, BlockState.air());
  }

  public static Block[] filledBuffer(int size, BlockState state) {
    Block[] blocks = new Block[size];
    for (int i = 0; i < size; i++) {
      blocks[i] = new Block(BlockPosition.of(0, 0, 0), state);
    }
    return blocks;
  }
}