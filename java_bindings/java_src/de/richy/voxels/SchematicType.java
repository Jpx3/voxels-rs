package de.richy.voxels;

public enum SchematicType {
  VXL("vxl", true),
  LITEMATIC("litematic", true),
  MCEDIT("mcedit", true),
  MOJANG("mojang", false),
  SPONGE("sponge", true),
  UNKNOWN(null, false)
  ;

  private final String typeName;
  private final boolean writerRequiresBoundary;

  SchematicType(String typeName, boolean writerRequiresBoundary) {
    this.typeName = typeName;
    this.writerRequiresBoundary = writerRequiresBoundary;
  }

  public String typeName() {
    return typeName;
  }

  public boolean writerRequiresBoundary() {
    return writerRequiresBoundary;
  }

	static {
	  Voxels.initialize();
  }
}