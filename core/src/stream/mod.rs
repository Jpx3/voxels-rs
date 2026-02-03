pub mod mojang_reader;
pub mod mojang_writer;
mod nbt_reader;
mod sponge;


use crate::common::{AxisOrder, Block};
use crate::store::blockstore::BlockStore;

pub trait SchematicInputStream {
    fn read(& mut self, buffer: &mut Vec<Block>, offset: usize, length: usize)
         -> Result<Option<usize>, String>;

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
}

/*
    Don't forget to call complete() after write_all() or write() to finalize the stream.
 */
pub trait SchematicOutputStream {
    fn write(&mut self, blocks: &[Block]) -> Result<usize, String>;

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

    fn complete(&mut self) -> Result<(), String>;
}
