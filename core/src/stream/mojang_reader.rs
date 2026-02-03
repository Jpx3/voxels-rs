use crate::common::{AxisOrder, Block, BlockState, Region};
use crate::store::blockstore::BlockStore;
use crate::store::blockstore::LazyPaletteBlockStoreWrapper;
use crate::store::blockstore::PagedBlockStore;
use crate::stream::SchematicInputStream;
use fastnbt::stream::{Parser, Value};
use fastnbt::Tag;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MojangSchematicInputStream<R: std::io::Read> {
    parser: Parser<R>,
    size_x: usize, size_y: usize, size_z: usize,
    header_read: bool,
    lazy_palette: LazyPalette
}

pub struct LazyPalette {
    blocks: Option<LazyPaletteBlockStoreWrapper>,
    current_index: usize,
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    pub fn new(inner: R) -> Self {
        Self {
            parser: Parser::new(inner),
            header_read: false,
            size_x: 0, size_y: 0, size_z: 0,
            lazy_palette: LazyPalette {
                blocks: None,
                current_index: 0
            }
        }
    }
}

fn poll_size(
    reader: &mut Parser<impl std::io::Read>
) -> Result<(usize, usize, usize), String> {
    let mut seen = 0;
    let mut x: Option<usize> = None;
    let mut y : Option<usize> = None;
    let mut z : Option<usize> = None;
    loop {
        match reader.next() {
            Ok(value) => match value {
                Value::Int(name, val) => {
                    if seen >= 3 {
                        Err("Too many size entries in schematic".to_string())?;
                    }
                    if let Some(name) = name {
                        match name.as_str() {
                            "Width" => x = Some(val as usize),
                            "Height" => y = Some(val as usize),
                            "Length" => z = Some(val as usize),
                            _ => {
                                return Err(format!("Unexpected size entry name: {}", name));
                            }
                        }
                    } else if seen < 3 {
                        match seen {
                            0 => x = Some(val as usize),
                            1 => y = Some(val as usize),
                            2 => z = Some(val as usize),
                            _ => {
                                return Err("Too many unnamed size entries in schematic".to_string());
                            }
                        }
                    }
                    seen += 1;
                }
                Value::ListEnd => {
                    break;
                }
                Value::CompoundEnd => {
                    break;
                },
                _ => {
                    return Err(format!("Unexpected NBT value while reading size: {:?}", value));
                }
            },
            Err(e) => {
                return Err(format!("Error reading NBT: {}", e));
            }
        }
    }
    if let (Some(x), Some(y), Some(z)) = (x, y, z) {
        Ok((x, y, z))
    } else {
        Err("Failed to read size from schematic".to_string())
    }
}

impl<R: std::io::Read> SchematicInputStream for MojangSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, _offset: usize, length: usize) -> Result<Option<usize>, String> {
        if !self.header_read {
            self.header_read = true;

            match self.read_schematic_header() {
                Ok(_) => {}
                Err(e) => {
                    return Err(format!("Error reading schematic header: {}", e));
                }
            }
        }
        if let Some(wrapper) = &self.lazy_palette.blocks {
            let iter = wrapper
                .iter(AxisOrder::XYZ)
                .skip(self.lazy_palette.current_index);
            let mut read_blocks = 0;
            for (i, block_pos) in iter.enumerate() {
                if i >= length { break; }
                let block_state = wrapper.block_at(&block_pos)?;
                match block_state {
                    Some(state) => {
                        buffer.push(Block::new(state, block_pos));
                    }
                    None => {
                        // buffer[offset + i] = Block::new(&BlockState::air_state_ref(), block_pos);
                        buffer.push(Block::new(BlockState::air_arc(), block_pos));
                    }
                }
                read_blocks += 1;
            }
            if read_blocks == 0 {
                return Ok(None);
            }
            self.lazy_palette.current_index += read_blocks;
            return Ok(Some(read_blocks));
        }
        Ok(None)
    }
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    fn read_schematic_header(&mut self) -> Result<(), String> {
        loop {
            match self.parser.next() {
                Ok(value) => {
                    match value {
                        Value::ByteArray(_, _) => {
                            Err("Unexpected ByteArray".to_string())?;
                        }
                        Value::List(ref name, typus, num) => {
                            if let (Some(name), Tag::Int, 3) = (name, typus, num) {
                                if name.eq_ignore_ascii_case("Size") {
                                    let (x, y, z) = poll_size(&mut self.parser)?;
                                    self.size_x = x;
                                    self.size_y = y;
                                    self.size_z = z;
                                } else {
                                    Err(format!("Unexpected list name: {}", name))?;
                                }
                            } else if let (Some(name), Tag::Compound) = (name, typus) {
                                if self.lazy_palette.blocks.is_none() {
                                    self.lazy_palette.blocks = Some(
                                        LazyPaletteBlockStoreWrapper::empty_resizable_from_size(
                                            self.size_x.max(16),
                                            self.size_y.max(16),
                                            self.size_z.max(16)
                                        ));
                                }
                                if name == "palette" {
                                    self.extract_palette_from_nbt_stream()?;
                                } else if name == "blocks" {
                                    self.read_blocks_from_nbt_stream()?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return if e.is_eof() {
                        Ok(())
                    } else {
                        Err(format!("Error reading NBT: {}", e))
                    }
                }
            }
        }
    }

    fn extract_palette_from_nbt_stream(&mut self) -> Result<(), String> {
        let mut palette: HashMap<isize, Arc<BlockState>> = HashMap::new();
        // palette.insert(
        //     0, BlockState::air_arc()
        // );
        let mut type_name = String::new();
        let mut properties = HashMap::<String, String>::new();
        let mut depth = 1;
        loop {
            match self.parser.next() {
                Ok(value) => {
                    match value {
                        Value::String(name, value) => {
                            if depth == 2 {
                                if let Some(name) = name {
                                    if name == "Name" {
                                        type_name = value.clone();
                                    } else {
                                        Err(format!("Unexpected palette entry name at depth 1: {}", name))?;
                                    }
                                } else {
                                    Err("Unnamed palette entry at depth 1".to_string())?;
                                }
                            } else if depth == 3 {
                                if let Some(name) = name {
                                    properties.insert(name, value);
                                } else {
                                    Err("Unnamed property in palette".to_string())?;
                                }
                            }
                        }
                        Value::Compound(Some(name)) => {
                            if name == "Properties" {
                                depth += 1;
                            }
                        }
                        Value::Compound(None) => {
                            depth += 1;
                        }
                        Value::CompoundEnd => {
                            depth -= 1;
                            if depth == 1 {
                                let block_state = BlockState::from_name_and_properties(&type_name, &properties);
                                let index = palette.len() as isize;
                                palette.insert(index, Arc::new(block_state));
                                properties.clear();
                            }
                            if depth == 0 {
                                break;
                            }
                        }
                        Value::ListEnd => {
                            break
                        }
                        _ => {
                            // print!("Unexpected palette NBT value: {:?}\n", value);
                            Err("Unexpected palette NBT value".to_string())?;
                            break;
                        }
                    }
                }
                Err(e) => {
                    Err(format!("Error reading NBT in palette: {}", e))?;
                }
            }
        }
        let palette1 = &mut self.lazy_palette;
        if let Some(wrapper) = &mut palette1.blocks {
            wrapper.set_actual_palette(palette);
        }
        Ok(())
    }

    fn read_blocks_from_nbt_stream(&mut self) -> Result<i32, String> {
        let mut depth = 1;
        let mut block_count = 0;
        let mut current_index = 0;
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;

        loop {
            match self.parser.next() {
                Ok(value) => {
                    match value {
                        Value::List(Some(_name), Tag::Int, 3) => {
                            depth += 1;
                        }
                        Value::ListEnd => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        Value::Int(None, val) => {
                            if depth > 1 {
                                match current_index {
                                    0 => { x = val as usize; },
                                    1 => { y = val as usize; },
                                    2 => { z = val as usize; },
                                    _ => {
                                        Err("Too many int values in block position".to_string())?;
                                    }
                                }
                                current_index += 1;
                            } else {
                                Err("Unexpected int value at top level of blocks".to_string())?;
                            }
                        }
                        Value::Int(Some(name), val) => {
                            if name == "state" {
                                // we have a block state index
                                block_count += 1;
                                current_index = 0;
                                if let Some(wrapper) = &mut self.lazy_palette.blocks {
                                    wrapper.set_unknown_block_at(x as i32, y as i32, z as i32, val as isize)?;
                                    // print!("Set block at ({}, {}, {}) to state {}\n", x, y, z, val);
                                } else {
                                    Err("Palette not initialized when reading blocks".to_string())?;
                                }
                            } else {
                                Err(format!("Unexpected int name in block: {}", name))?;
                            }
                        }
                        _ => {
                            // Err(format!("Unexpected NBT value in blocks: {:?}", value))?;
                        }
                    }
                }
                Err(e) => {
                    Err(format!("Error reading NBT in blocks: {}", e))?;
                }
            }
        }
        Ok(block_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_mojang_schematic_input_stream() {
        let file = File::open("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.schem").expect("Failed to open schematic file");
        let reader = BufReader::new(file);
        let mut gz_decoder = GzDecoder::new(reader);
        let mut schematic_stream = MojangSchematicInputStream::new(&mut gz_decoder);
        let mut block_store = PagedBlockStore::empty_resizable();
        schematic_stream.read_to_end(&mut block_store).expect("Failed to read schematic to end");
        let mut non_air_blocks = 0;
        for x in block_store.iterate_blocks(AxisOrder::XYZ) {
            if !x.1.clone().or_else(|| Some(BlockState::air_arc())).unwrap().is_air() {
                println!("Block at position {:?} with state {:?}", x.0, x.1);
                non_air_blocks += 1;
            }
        }
        println!("Total non-air blocks: {}", non_air_blocks);
    }
}