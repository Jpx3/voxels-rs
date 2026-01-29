package de.richy.voxels;

public class BlockPosition {
  public static long refCnt = 0;

  private final int x;
  private final int y;
  private final int z;

  private BlockPosition(int x, int y, int z) {
    this.x = x;
    this.y = y;
    this.z = z;
    refCnt++;
  }

  public int x() {
    return x;
  }

  public int y() {
    return y;
  }

  public int z() {
    return z;
  }

  public String toString() {
    return "BlockPosition{" + "x=" + x + ", y=" + y + ", z=" + z + '}';
  }
}