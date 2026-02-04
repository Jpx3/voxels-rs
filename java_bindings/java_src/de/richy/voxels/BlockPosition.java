package de.richy.voxels;

public class BlockPosition {
  private int x;
  private int y;
  private int z;

  private BlockPosition(int x, int y, int z) {
    this.x = x;
    this.y = y;
    this.z = z;
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

  public void setX(int x) {
    this.x = x;
  }

  public void setY(int y) {
    this.y = y;
  }

  public void setZ(int z) {
    this.z = z;
  }

  public static BlockPosition of(int x, int y, int z) {
    return new BlockPosition(x, y, z);
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