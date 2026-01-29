package de.richy.voxels;

import java.util.Map;

import java.util.Map;
import java.util.HashMap;

public class BlockState {
  public static long refCnt = 0;

  private final String typeName;
  private final Map<String, String> properties;

  private BlockState(String typeName, Map<String, String> properties) {
    this.typeName = typeName;
    this.properties = new HashMap<>(properties);
    refCnt++;
  }

  public String typeName() {
    return typeName;
  }

  public Map<String, String> properties() {
    return properties;
  }

  public String toString() {
    return "BlockState{typeName=" + typeName + ", properties=" + properties + "}";
  }
}