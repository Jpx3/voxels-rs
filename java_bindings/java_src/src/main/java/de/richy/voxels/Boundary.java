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
    return minX + dX - 1;
  }
  
  public int maxY() {
    return minY + dY - 1;
  }
  
  public int maxZ() {
    return minZ + dZ - 1;
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
    int newMaxX = Math.max(maxX(), x);
    int newMaxY = Math.max(maxY(), y);
    int newMaxZ = Math.max(maxZ(), z);
    return new Boundary(
      newMinX, newMinY, newMinZ,
      newMaxX - newMinX + 1,
      newMaxY - newMinY + 1,
      newMaxZ - newMinZ + 1
    );
  }

  public static Boundary fromMinAndMax(
    int minX, int minY, int minZ,
    int maxX, int maxY, int maxZ
  ) {
    return new Boundary(
      minX, minY, minZ,
      maxX - minX + 1, maxY - minY + 1, maxZ - minZ + 1
    );
  }
}