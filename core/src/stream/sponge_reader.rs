use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::store::blockstore::LazyPaletteBlockStoreWrapper;
use fastnbt::Value;
use std::collections::HashMap;
use std::io::Read;
use std::ops::Deref;
use std::sync::Arc;
use crate::stream::stream::SchematicInputStream;

pub struct SpongeSchematicInputStream<R: Read> {
    reader: R,
    header_read: bool,
    blocks: Option<LazyPaletteBlockStoreWrapper>,
    read_blocks: usize,
    boundary: Option<Boundary>,
}

impl<R: Read> SchematicInputStream for SpongeSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, _offset: usize, length: usize) -> Result<Option<usize>, String> {
        if !self.header_read {
            self.read_header()?;
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
            self.read_header()?;
        }
        Ok(self.boundary)
    }
}


impl<R: Read> SpongeSchematicInputStream<R> {
    pub fn new(reader: R) -> Self {
        SpongeSchematicInputStream {
            reader,
            header_read: false,
            blocks: None,
            read_blocks: 0,
            boundary: None,
        }
    }

    fn read_header(&mut self) -> Result<(), String> {
        let result: Value = fastnbt::from_reader(&mut self.reader).map_err(|e| format!("Sponge: Failed to read NBT data: {}", e))?;
        if let Value::Compound(root) = result {
            if !root.contains_key("Schematic") {
                return Err("Sponge: Missing 'Schematic' tag".into());
            }
            let schematic_value = &root["Schematic"];
            if let Value::Compound(schematic) = schematic_value {
                let height = match schematic.get("Height") {
                    Some(Value::Short(v)) => *v as i32,
                    _ => return Err("Sponge: Missing or invalid 'Height' tag".into()),
                };
                let length = match schematic.get("Length") {
                    Some(Value::Short(v)) => *v as i32,
                    _ => return Err("Sponge: Missing or invalid 'Length' tag".into()),
                };
                let width = match schematic.get("Width") {
                    Some(Value::Short(v)) => *v as i32,
                    _ => return Err("Sponge: Missing or invalid 'Width' tag".into()),
                };
                self.boundary = Some(Boundary::new(0, 0, 0, width, height, length));
                self.blocks = Some(LazyPaletteBlockStoreWrapper::empty_fixed_from_size(
                    width as usize, height as usize, length as usize,
                ));
                self.process_palette(schematic).map_err(|e| format!("Sponge: Failed to process palette: {}", e))?;
                self.process_blocks(schematic).map_err(|e| format!("Sponge: Failed to process blocks: {}", e))?;
            } else {
                return Err("Sponge: Missing or invalid 'Schematic' tag".into());
            }
            self.header_read = true;
            Ok(())
        } else {
            Err("Sponge: Root tag is not a Compound".into())
        }
    }

    fn process_palette(&mut self, schematic: &HashMap<String, Value>) -> Result<(), String> {
        if self.blocks.is_none() {
            return Err("Sponge: Blocks store not initialized before processing palette".into());
        }
        let blocks = self.blocks.as_mut().unwrap();
        let palette_tag = if schematic.contains_key("Blocks") {
            match &schematic["Blocks"] {
                Value::Compound(content) => {
                    &content["Palette"]
                }
                _ => return Err("Sponge: 'Blocks' tag is not a Compound".into()),
            }
        } else {
            &schematic["Palette"]
        };

        let palette_compound = match palette_tag {
            Value::Compound(map) => map,
            _ => return Err("Sponge: 'Palette' tag is not a Compound".into()),
        };
        let mut palette: HashMap<isize, Arc<BlockState>> = HashMap::new();
        for x in palette_compound {
            let name = x.0;
            let state = match &x.1 {
                Value::Int(v) => *v,
                _ => return Err("Sponge: Palette entry value is not an Int".into()),
            };
            let block_state = Arc::new(BlockState::from_string(name.clone())?);
            palette.insert(state as isize, block_state);
        }
        blocks.set_actual_palette(palette);
        Ok(())
    }

    fn process_blocks(&mut self, schematic: &HashMap<String, Value>) -> Result<(), String> {
        if self.boundary.is_none() {
            return Err("Sponge: Boundary not set before processing blocks".into());
        }
        if self.blocks.is_none() {
            return Err("Sponge: Blocks store not initialized before processing blocks".into());
        }

        let block_tag = if schematic.contains_key("Blocks") {
            match &schematic["Blocks"] {
                Value::Compound(content) => {
                    &content["Data"]
                }
                _ => return Err("Sponge: 'Blocks' tag is not a Compound".into()),
            }
        } else {
            &schematic["BlockData"]
        };
        match block_tag {
            Value::ByteArray(byte_array) => {
                let bytes = byte_array.deref();
                let bytes = bytes.iter().map(|b| *b as u8).collect::<Vec<u8>>();
                let block_states = self.read_var_int_array(&bytes)?;
                let boundary = self.boundary.unwrap();
                let mut block_iter = boundary.iter(AxisOrder::YZX);
                for (_, state_index) in block_states.iter().enumerate() {
                    let pos = block_iter.next().ok_or("Sponge: Boundary size mismatch (iterator exhausted before stream)")?;
                    self.blocks.as_mut().unwrap().set_unknown_block(
                        &pos, *state_index as isize
                    ).map_err(|e| format!("Sponge: Failed to copy block at pos {:?}: {}", pos, e))?;
                }
                Ok(())
            },
            _ => {
                Err("Sponge: 'BlockData' tag is not a ByteArray".into())
            }
        }
    }

    fn read_var_int_array(&mut self, data: &[u8]) -> Result<Vec<i32>, String> {
        let mut integers = Vec::new();
        let mut index = 0;
        while index < data.len() {
            let mut value = 0;
            let mut shift = 0;
            loop {
                if index >= data.len() {
                    return Err("Sponge: VarInt array ended unexpectedly".into());
                }
                let byte = data[index];
                index += 1;
                value |= ((byte & 0x7F) as i32) << shift;
                if (byte & 0x80) == 0 {
                    break;
                }
                shift += 7;
                if shift > 35 {
                    return Err("Sponge: VarInt is too big".into());
                }
            }
            integers.push(value);
        }
        Ok(integers)
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use flate2::read::GzDecoder;
    use crate::common::{Block, BlockPosition, BlockState};
    use crate::stream::sponge_reader::SpongeSchematicInputStream;
    use crate::stream::stream::SchematicInputStream;

    fn create_test_schematic() -> Vec<Block> {
        let mut blocks = Vec::new();
        let trunk_x = 8;
        let trunk_z = 8;
        let trunk_height = 5;
        let leaf_start = 3;
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let dx = (x as i32 - trunk_x).abs();
                    let dz = (z as i32 - trunk_z).abs();
                    let dist_sq = dx * dx + dz * dz;
                    let mut block_state = BlockState::air();
                    if dx == 0 && dz == 0 && y < trunk_height {
                        block_state = BlockState::from_string(format!("minecraft:oak_log[axis={}]", ["y", "x", "z"][y as usize % 3])).unwrap();
                    } else if y >= leaf_start && y <= trunk_height + 1 {
                        let radius = if y == trunk_height + 1 { 2 } else { 3 };
                        if dist_sq < radius * radius && !(dx == radius - 1 && dz == radius - 1) {
                            block_state = BlockState::from_string("minecraft:oak_leaves[distance=1,persistent=true]".to_string()).unwrap();
                        }
                    }
                    if !block_state.is_air() {
                        blocks.push(Block {
                            position: BlockPosition::new(x, y, z),
                            state: Arc::new(block_state),
                        });
                    }
                }
            }
        }
        blocks
    }

    #[test]
    fn test_sponge_reader() {
        const TREE_SCHEMATIC: &[u8] = include_bytes!("test_schematics/tree.sponge");
        let reader = std::io::Cursor::new(TREE_SCHEMATIC);
        let reader = GzDecoder::new(reader);
        let mut sponge_reader = SpongeSchematicInputStream::new(reader);
        let read_blocks = sponge_reader.read_to_end_into_vec().unwrap();
        let expected_blocks = create_test_schematic();
        assert_eq!(read_blocks.len(), expected_blocks.len());
        for expected in expected_blocks.clone() {
            let found = read_blocks.iter().find(|b| b.position == expected.position);
            assert!(found.is_some(), "Expected block at position {:?} not found", expected.position);
            let found_block = found.unwrap();
            assert_eq!(found_block.state.name(), expected.state.name(), "Block state name mismatch at position {:?}", expected.position);
            assert_eq!(found_block.state.properties(), expected.state.properties(), "Block state properties mismatch at position {:?}", expected.position);
        }

        for block in read_blocks {
            let expected = expected_blocks.iter().find(|b| b.position == block.position).unwrap();
            assert_eq!(block.state.name(), expected.state.name(), "Block state name mismatch at position {:?}", block.position);
            assert_eq!(block.state.properties(), expected.state.properties(), "Block state properties mismatch at position {:?}", block.position);
        }
    }
}