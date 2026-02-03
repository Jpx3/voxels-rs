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
  
  public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    BlockPosition that = (BlockPosition) o;
    if (x != that.x) return false;
    if (y != that.y) return false;
    return z == that.z;
  }
  
  public int hashCode() {
    int result = x;
    result = 31 * result + y;
    result = 31 * result + z;
    return result;
  }

  public String toString() {
    return "BlockPosition{" + "x=" + x + ", y=" + y + ", z=" + z + '}';
  }
}