use std::collections::HashMap;
use fastnbt::stream::{Error, ErrorKind, Parser, Value};
use fastnbt::Tag;
use crate::common::{AxisOrder, Block, BlockPosition, BlockState, Boundary, Region, Schematic};
use crate::store::blockstore::{BlockStore, LazyPaletteBlockStoreWrapper, PagedBlockStore};
use crate::stream::SchematicInputStream;

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
                            _ => {}
                        }
                    } else if seen < 3 {
                        match seen {
                            0 => x = Some(val as usize),
                            1 => y = Some(val as usize),
                            2 => z = Some(val as usize),
                            _ => {}
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
    fn read<'a>(&'a mut self, buffer: &mut [Block<'a>], offset: usize, length: usize) -> Result<Option<usize>, String> {
        if !self.header_read {
            self.header_read = true;
            self.read_schematic_header().expect("Failed to read header");
        }

        if let Some(wrapper) = &self.lazy_palette.blocks {
            // Create the iterator on the fly, skip to the current progress
            let iter = wrapper.iter(AxisOrder::XYZ).skip(self.lazy_palette.current_index);

            let mut read_blocks = 0;
            for (i, block_pos) in iter.enumerate() {
                if i >= length { break; }
                let block_state = wrapper.block_at(
                    &block_pos
                )?;
                match block_state {
                    Some(state) => {
                        buffer[offset + i] = Block::new(&state, block_pos);
                    }
                    None => {
                        // buffer[offset + i] = Block::new(&BlockState::air(), block_pos);
                    }
                }

                read_blocks += 1;
            }

            self.lazy_palette.current_index += read_blocks;
            return Ok(Some(read_blocks));
        }

        Ok(Some(0))
    }
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    fn read_schematic_header(&mut self) -> Result<(), String> {
        loop {
            match self.parser.next() {
                Ok(value) => {
                    match value {
                        Value::ByteArray(_, _) => {
                            println!("Big array read, returning data...");
                        }
                        Value::List(ref name, typus, num) => {
                            if let (Some(name), Tag::Int, 3) = (name, typus, num) {
                                if name == "Size" {
                                    let (x, y, z) = poll_size(&mut self.parser)?;
                                    println!("Schematic size: {}x{}x{}", x, y, z);
                                    self.size_x = x;
                                    self.size_y = y;
                                    self.size_z = z;
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
                                    let block_count = self.read_blocks_from_nbt_stream()?;
                                    println!("Read {} blocks from schematic", block_count);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(format!("Error reading NBT: {}", e));
                }
            }
        }
    }

    fn extract_palette_from_nbt_stream(&mut self) -> Result<(), String> {
        // let
        let mut palette: HashMap<isize, BlockState> = HashMap::new();
        palette.insert(
            0, BlockState::air()
        );

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
                            if (name == "Properties") {
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
                                palette.insert(index, block_state);
                                print!("Palette entry: {} with properties {:?}\n", type_name, properties);
                                print!("Assigned index {}\n", index);
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
                            print!("Unexpected palette NBT value: {:?}\n", value);
                            break;
                        }
                    }
                }
                Err(e) => {
                    Err(format!("Error reading NBT in palette: {}", e))?;
                }
            }
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
                                    print!("Set block at ({}, {}, {}) to state {}\n", x, y, z, val);
                                }
                            } else {
                                Err(format!("Unexpected int name in block: {}", name))?;
                            }
                        }
                        _ => {}
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
    use std::fs::File;
    use std::io::BufReader;
    use flate2::read::GzDecoder;

    #[test]
    fn test_mojang_schematic_input_stream() {
        let file = File::open("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\mojang.schem").expect("Failed to open schematic file");
        let reader = BufReader::new(file);
        let mut gz_decoder = GzDecoder::new(reader);
        let mut schematic_stream = MojangSchematicInputStream::new(&mut gz_decoder);
        let air = BlockState::air();
        let mut buffer: [Block; 4096] = std::array::from_fn(|_| Block::new_at_zero(&air));
        let length = buffer.len();
        match schematic_stream.read(&mut buffer, 0, length) {
            Ok(Some(read_blocks)) => {
                println!("Read {} blocks from schematic", read_blocks);
            }
            Ok(None) => {
                println!("End of schematic reached");
            }
            Err(e) => {
                println!("Error reading schematic: {}", e);
            }
        }
    }
}