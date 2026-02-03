package de.richy.voxels;

import java.io.OutputStream;

public class BlockOutputStream implements AutoCloseable {
  private final long ptr = 0;

  public void write(Block[] blocks) {
    write(blocks, 0, blocks.length);
  }

  // Usually doesn't write the blocks to disk yet
  public native void write(Block[] blocks, int offset, int length);

  // Usually flushes the blocks to disk
  @Override
  public synchronized native void close();

  static {
	  Voxels.initialize();
  }
}