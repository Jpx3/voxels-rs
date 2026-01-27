mod mojang;
mod nbt_reader;
mod sponge;

use crate::common::{Block, BlockState};
use crate::store::blockstore::BlockStore;

pub trait SchematicInputStream {
    fn read<'a>(&'a mut self, buffer: &mut [Block<'a>], offset: usize, length: usize)
         -> Result<Option<usize>, String>;

    fn read_to_end(&mut self, store: &mut dyn BlockStore) -> Result<(), String> {
        let air_state = BlockState::air();
        loop {
            let mut buffer: [Block; 4096] = std::array::from_fn(|_| Block::new_at_zero(&air_state));
            let length = buffer.len();
            if let Some(read_blocks) = self.read(&mut buffer, 0, length)? {
                store.insert(&buffer, 0, read_blocks)?;
            } else {
                break
            }
        }
        Ok(())
    }
}
