package de.richy.voxels;

public record Boundary(
  int minX, int minY, int minZ,
  int dX, int dY, int dZ
) {
  public boolean contains(
    int x, int y, int z
  ) {
    return x >= minX && x < minX + dX &&
       y >= minY && y < minY + dY &&
       z >= minZ && z < minZ + dZ;
  }
  
  public int maxX() {
    return minX + dX;
  }
  
  public int maxY() {
    return minY + dY;
  }
  
  public int maxZ() {
    return minZ + dZ;
  }

  public Boundary expandToInclude(
    int x, int y, int z
  ) {
    if (contains(x, y, z)){
      return this;
    }
    int newMinX = Math.min(minX, x);
    int newMinY = Math.min(minY, y);
    int newMinZ = Math.min(minZ, z);
    int newMaxX = Math.max(maxX(), x + 1);
    int newMaxY = Math.max(maxY(), y + 1);
    int newMaxZ = Math.max(maxZ(), z + 1);
    return new Boundary(
      newMinX,
      newMinY,
      newMinZ,
      newMaxX - newMinX,
      newMaxY - newMinY,
      newMaxZ - newMinZ
    );
  }
  
  public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    Boundary boundary = (Boundary) o;
    return minX == boundary.minX &&
        minY == boundary.minY &&
        minZ == boundary.minZ &&
        dX == boundary.dX &&
        dY == boundary.dY &&
        dZ == boundary.dZ;
  }
  
  public int hashCode() {
    int result = Integer.hashCode(minX);
    result = 31 * result + Integer.hashCode(minY);
    result = 31 * result + Integer.hashCode(minZ);
    result = 31 * result + Integer.hashCode(dX);
    result = 31 * result + Integer.hashCode(dY);
    result = 31 * result + Integer.hashCode(dZ);
    return result;
  }
  
  public String toString() {
    return "Boundary{" +
        "minX=" + minX +
        ", minY=" + minY +
        ", minZ=" + minZ +
        ", dX=" + dX +
        ", dY=" + dY +
        ", dZ=" + dZ +
        '}';
  }
}