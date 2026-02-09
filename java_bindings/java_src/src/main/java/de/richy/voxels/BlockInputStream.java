package de.richy.voxels;

import java.io.InputStream;

public class BlockInputStream implements AutoCloseable {
  private final long ptr = 0;

  public int read(Block[] blocks) {
    return read(blocks, 0, blocks.length);
  }

  public native int read(Block[] blocks, int offset, int length);

  public native Boundary boundary();

  @Override
  public synchronized native void close();
  
  static {
	Voxels.initialize();
  }
}