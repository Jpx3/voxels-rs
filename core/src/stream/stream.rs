use crate::common::{AxisOrder, Block, Boundary};
use crate::store::blockstore::BlockStore;

/// A stream for reading schematic data block by block.
pub trait SchematicInputStream {
    /// Reads up to `length` blocks into the provided buffer starting from `offset`.
    /// Returns the number of blocks read, or `None` if the end of the stream is reached.
    fn read(& mut self, buffer: &mut Vec<Block>, offset: usize, length: usize)
            -> Result<Option<usize>, String>;

    /// Reads all blocks from the input stream into the given BlockStore.
    /// This method handles buffering internally for efficiency.
    fn read_to_end(&mut self, store: &mut dyn BlockStore) -> Result<(), String> {
        loop {
            let mut blocks = Vec::new();
            if let Some(read_blocks) = self.read(&mut blocks, 0, 4096)? {
                store.insert(&blocks, 0, read_blocks)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Retrieves the boundary information of the schematic, if available.
    fn boundary(&mut self) -> Result<Option<Boundary>, String>;
}

/// A stream for writing schematic data block by block.
/// 
/// Make sure to call `complete()` after `write_all()` or `write()` to finalize the stream.
pub trait SchematicOutputStream {
    /// Writes a slice of blocks to the output stream.
    /// Returns the number of blocks written.
    fn write(&mut self, blocks: &[Block]) -> Result<usize, String>;

    /// Writes all blocks from the given BlockStore to the output stream.
    /// This method handles buffering internally for efficiency.
    fn write_all(&mut self, blocks: &mut dyn BlockStore) -> Result<(), String> {
        let iter = blocks.iterate_blocks(AxisOrder::XYZ);
        let chunk_size = 4096;
        let mut buffer = Vec::with_capacity(chunk_size);
        for (pos, block_state) in iter {
            if let Some(bs) = block_state {
                buffer.push(Block::new(bs, pos));
                if buffer.len() >= chunk_size {
                    self.write(&buffer)?;
                    buffer.clear();
                }
            }
        }
        if !buffer.is_empty() {
            self.write(&buffer)?;
        }
        Ok(())
    }

    /// Completes the output stream, finalizing any necessary data.
    /// This must be called after all writes are done.
    fn complete(&mut self) -> Result<(), String>;
}
