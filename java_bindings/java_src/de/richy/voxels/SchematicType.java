package de.richy.voxels;

public enum SchematicType {
    VXL("vxl"),
    LITEMATIC("litematic"),
    MCEDIT("mcedit"),
    MOJANG("mojang"),
    SPONGE_V1("sponge_v1"),
    SPONGE_V2("sponge_v2"),
    SPONGE_V3("sponge_v3");
    ;

    private final String typeName;

    SchematicType(String typeName) {
        this.typeName = typeName;
    }

    public String typeName() {
        return typeName;
    }

	static {
	    Voxels.initialize();
    }
}