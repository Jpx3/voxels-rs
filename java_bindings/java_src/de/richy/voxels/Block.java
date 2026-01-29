package de.richy.voxels;

public class Block {
  private final BlockPosition position;
  private final BlockState state;

  private Block(BlockPosition position, BlockState state) {
    this.position = position;
    this.state = state;
  }

  public BlockPosition position() {
    return position;
  }

  public BlockState state() {
    return state;
  }

  public String toString() {
    return "Block{position=" + position + ", state=" + state + "}";
  }
}