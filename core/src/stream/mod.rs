pub mod mojang;
mod nbt_reader;
mod sponge;

use crate::common::{Block, BlockState};
use crate::store::blockstore::BlockStore;

pub trait SchematicInputStream {
    fn read<'a>(&'a mut self, buffer: &mut Vec<Block<'a>>, offset: usize, length: usize)
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
