use crate::common::{Block, Boundary, Schematic};
use crate::store::blockstore::{BlockStore, PagedBlockStore};

pub trait SchematicInputStream {
    fn read(&mut self, buffer: &mut [Block], offset: usize, length: usize)
        -> Result<Option<usize>, String> ;

    fn read_to_end(&mut self, store: &mut dyn BlockStore) -> Result<(), String> {
        let mut buffer: [Block; 4096] = std::array::from_fn(|_| Block::air());
        loop {
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

pub struct MojangSchematicInputStream<R: std::io::Read> {
    inner: R,
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: std::io::Read> SchematicInputStream for MojangSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut [Block], offset: usize, length: usize) -> Result<Option<usize>, String> {
        todo!()
    }
}