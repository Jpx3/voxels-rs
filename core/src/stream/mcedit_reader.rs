use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::store::blockstore::{BlockStore, PagedBlockStore};
use crate::stream::legacy_ids::{convert_legacy_data_to_modern_properties, get_legacy_type};
use crate::stream::stream::SchematicInputStream;
use fastnbt::Value;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

pub struct MCEditSchematicInputStream<R: Read> {
    reader: R,
    header_read: bool,
    blocks: Option<Box<dyn BlockStore>>,
    read_blocks: usize,
    boundary: Option<Boundary>,
}

impl<R: Read> MCEditSchematicInputStream<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            header_read: false,
            blocks: None,
            read_blocks: 0,
            boundary: None,
        }
    }

    fn read_nbt(&mut self) -> Result<(), String> {
        if self.header_read {
            return Err("Sponge: NBT header has already been read".to_string());
        }
        if self.blocks.is_some() {
            return Err("Sponge: Blocks have already been read, cannot read NBT header".to_string());
        }

        let result: Value = fastnbt::from_reader(&mut self.reader).map_err(|e| format!("Sponge: Failed to read NBT data: {}", e))?;

        if let Value::Compound(root) = result {
            let width = if let Some(Value::Short(w)) = root.get("Width") {
                *w as usize
            } else {
                return Err("Sponge: Missing or invalid 'Width' tag".to_string());
            };
            let height = if let Some(Value::Short(h)) = root.get("Height") {
                *h as usize
            } else {
                return Err("Sponge: Missing or invalid 'Height' tag".to_string());
            };
            let length = if let Some(Value::Short(l)) = root.get("Length") {
                *l as usize
            } else {
                return Err("Sponge: Missing or invalid 'Length' tag".to_string());
            };
            let blocks = if let Some(Value::ByteArray(blocks)) = root.get("Blocks") {
                blocks.clone()
            } else {
                return Err("Sponge: Missing or invalid 'Blocks' tag".to_string());
            };
            let data = if let Some(Value::ByteArray(data)) = root.get("Data") {
                data.clone()
            } else {
                return Err("Sponge: Missing or invalid 'Data' tag".to_string());
            };
            let add_blocks = if let Some(Value::ByteArray(add_blocks)) = root.get("AddBlocks") {
                Some(add_blocks.clone())
            } else {
                None
            };
            let specified_block_ids: Option<HashMap<i32, String>> = if let Some(Value::Compound(block_ids)) = root.get("BlockIds") {
                Some(block_ids.iter().filter_map(|(k, v)| {
                    if let Value::String(s) = v {
                        s.parse::<i32>().ok().map(|id| (id, k.clone()))
                    } else {
                        None
                    }
                }).collect())
            } else {
                None
            };

            self.boundary = Some(Boundary::new_from_size(width as i32, height as i32, length as i32));
            self.blocks = Some(Box::new(PagedBlockStore::new_for_fixed_boundary(self.boundary.unwrap().clone())));
            let block_store = self.blocks.as_mut().unwrap();

            // "blocks" to u8 array, then use read_block_id to get the block id for each position in the boundary
            let block_ids = blocks.as_ref().iter().map(|b| *b as u8).collect::<Vec<u8>>();
            let add_blocks = add_blocks.as_ref().map(|ab| ab.iter().map(|b| *b as u8).collect::<Vec<u8>>());
            let block_data = data.as_ref().iter().map(|b| *b as u8).collect::<Vec<u8>>();

            let mut block_state_cache = HashMap::new();

            let mut idx: usize = 0;
            for position in self.boundary.unwrap().iter(AxisOrder::YZX) {
                let block_id = Self::read_block_id(&block_ids, idx, add_blocks.as_deref());
                let block_data = block_data[idx] & 0x0F;

                let block_cache_key = block_id << 4 | block_data as i32;

                if block_id != 0 {
                    if let None = block_state_cache.get(&block_cache_key) {
                        let block_name = if let Some(specified_block_ids) = &specified_block_ids {
                            specified_block_ids.get(&block_id).cloned()
                        } else {
                            None
                        }.or_else(|| get_legacy_type(block_id as usize, block_data));
                        if let Some(block_name) = block_name {
                            block_state_cache.insert(block_cache_key, Arc::new(BlockState::from_string(block_name)?));
                        } else {
                            convert_legacy_data_to_modern_properties(block_id as usize, block_data).map(|state| {
                                println!("Sponge: Converted legacy block ID {} with data {} to modern state {:?}", block_id, block_data, state);
                                block_state_cache.insert(block_cache_key, Arc::new(state));
                            }).unwrap_or_else(|| {
                               println!("Sponge: Warning - Unrecognized block ID {} with data {}, treating as air", block_id, block_data);
                                block_state_cache.insert(block_cache_key, Arc::new(BlockState::air()));
                            });
                        }
                    }
                    let block_state = block_state_cache.get(&block_cache_key).unwrap().clone();
                    block_store.set_block_at(&position, block_state)?;
                }
                idx += 1;
            }
        } else {
            return Err("Sponge: Root NBT tag is not a compound".to_string());
        }
        Ok(())
    }

    #[inline]
    fn read_block_id(block_ids: &[u8], idx: usize, add_blocks: Option<&[u8]>) -> i32 {
        let mut id = block_ids[idx] as i32 & 0xFF;
        if let Some(add_blocks) = add_blocks {
            let add_blocks_index = idx / 2;
            let nibble = add_blocks[add_blocks_index] as i32 & 0xFF;
            if (idx & 1) == 0 {
                id |= (nibble & 0x0F) << 8;
            } else {
                id |= (nibble >> 4) << 8;
            }
        }
        id
    }
}


impl<R: Read> SchematicInputStream for MCEditSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, offset: usize, length: usize) -> Result<Option<usize>, String> {
        if !self.header_read {
            self.read_nbt()?;
            self.header_read = true;
        }
        if self.boundary.is_none() || self.blocks.is_none() {
            return Err("Sponge: Header not properly read".into());
        }
        let mut blocks_written = 0;
        let boundary = self.boundary.unwrap();
        let blocks_store = self.blocks.as_ref().unwrap();
        let mut block_iter = boundary.iter(AxisOrder::XYZ).skip(self.read_blocks);
        while blocks_written < length {
            let pos = match block_iter.next() {
                Some(p) => p,
                None => break,
            };
            match blocks_store.block_at(&pos)? {
                None => {}
                Some(block_state) => {
                    if !block_state.is_air() {
                        let block = Block {
                            position: pos,
                            state: Arc::clone(&block_state),
                        };
                        buffer.push(block);
                        blocks_written += 1;
                    }
                }
            }
            self.read_blocks += 1;
        }
        if blocks_written == 0 && length > 0 {
            Ok(None)
        } else {
            Ok(Some(blocks_written))
        }
    }

    fn boundary(&mut self) -> Result<Option<Boundary>, String> {
        if !self.header_read {
            self.read_nbt()?;
            self.header_read = true;
        }
        Ok(self.boundary.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::stream::mcedit_reader::MCEditSchematicInputStream;
    use crate::stream::stream::SchematicInputStream;
    use flate2::read::GzDecoder;

    #[test]
    fn test_mcedit_arbitrary_read() {
        const TEST_SCHEMATIC: &[u8] = include_bytes!("test_schematics/mcedit.schematic");
        let reader = std::io::Cursor::new(TEST_SCHEMATIC);
        let reader = GzDecoder::new(reader);
        let mut sponge_reader = MCEditSchematicInputStream::new(reader);
        let read_blocks = sponge_reader.read_to_end_into_vec().unwrap();
        assert!(!read_blocks.is_empty(), "Expected to read some blocks from the schematic");
        // for block in read_blocks {
        //     println!("Block at {:?} with state {:?}", block.position, block.state);
        // }
        // panic!("Abc")
    }
}
