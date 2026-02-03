package de.richy.voxels;

import java.util.Map;

import java.util.Map;
import java.util.HashMap;

public class BlockState {
  private static long __ref_cnt = 0;
  private final long __internal_id;

  private final String typeName;
  private final Map<String, String> properties;

  public BlockState(String typeName, Map<String, String> properties) {
    this.typeName = typeName;
    this.properties = new HashMap<>(properties);
    this.__internal_id = __ref_cnt++;
  }

  public String typeName() {
    return typeName;
  }

  public Map<String, String> properties() {
    return properties;
  }

  public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    BlockState that = (BlockState) o;
    if (!typeName.equals(that.typeName)) return false;
    return properties.equals(that.properties);
  }

  public int hashCode() {
    int result = typeName.hashCode();
    result = 31 * result + properties.hashCode();
    return result;
  }

  public String toString() {
    return "BlockState{typeName=" + typeName + ", properties=" + properties + "}";
  }
}