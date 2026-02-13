use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::store::blockstore::LazyPaletteBlockStoreWrapper;
use crate::stream::stream::SchematicInputStream;
use fastnbt::stream::{Parser, Value};
use fastnbt::Tag;
use std::collections::HashMap;
use std::rc::Rc;

pub struct MojangSchematicInputStream<R: std::io::Read> {
    parser: Parser<R>,
    size: (usize, usize, usize),
    header_read: bool,
    lazy_palette: LazyPalette,
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
            size: (0, 0, 0),
            lazy_palette: LazyPalette {
                blocks: None,
                current_index: 0,
            },
        }
    }

    fn ensure_header_read(&mut self) -> Result<(), String> {
        if !self.header_read {
            self.header_read = true;
            self.read_schematic_header()?;
        }
        Ok(())
    }
}

impl<R: std::io::Read> SchematicInputStream for MojangSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, _offset: usize, length: usize) -> Result<Option<usize>, String> {
        self.ensure_header_read()?;

        let wrapper = match &self.lazy_palette.blocks {
            Some(w) => w,
            None => return Err("Blocks not initialized".into()),
        };

        let iter = wrapper
            .iter(AxisOrder::XYZ)
            .skip(self.lazy_palette.current_index);

        let mut read_count = 0;
        let mut written_count = 0;

        for (_, pos) in iter.enumerate() {
            if written_count >= length { break; }

            if let Some(state) = wrapper.block_at(&pos)? {
                if !state.is_air() {
                    buffer.push(Block::new(state, pos));
                    written_count += 1;
                }
            }
            read_count += 1;
        }

        if read_count == 0 { return Ok(None); }
        self.lazy_palette.current_index += read_count;
        Ok(Some(written_count))
    }

    fn boundary(&mut self) -> Result<Option<Boundary>, String> {
        self.ensure_header_read()?;
        let (x, y, z) = self.size;

        if x > 0 && y > 0 && z > 0 {
            Ok(Some(Boundary::new(0, 0, 0, x as i32, y as i32, z as i32)))
        } else {
            Ok(None)
        }
    }
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    fn read_schematic_header(&mut self) -> Result<(), String> {
        let mut palette_found = false;
        let mut blocks_found = false;
        let mut size_found = false;
        loop {
            match self.parser.next() {
                Ok(Value::List(Some(name), tag, len)) => match name.to_lowercase().as_str() {
                    "size" if tag == Tag::Int && len == 3 => {
                        self.size = poll_size(&mut self.parser)?;
                        size_found = true;
                    }
                    "palette" if tag == Tag::Compound => {
                        self.ensure_blocks_initialized();
                        self.extract_palette_from_nbt_stream()?;
                        palette_found = true;
                    }
                    "blocks" if tag == Tag::Compound => {
                        self.ensure_blocks_initialized();
                        self.read_blocks_from_nbt_stream()?;
                        blocks_found = true;
                    }
                    _ => {}
                },
                Ok(Value::CompoundEnd) | Ok(Value::ListEnd) => continue,
                Ok(_) => {}
                Err(e) if e.is_eof() => break,
                Err(e) => return Err(format!("NBT Stream Error: {}", e)),
            }
        }
        if !size_found {
            return Err("Mojang: Size not found in header".into());
        }
        if !palette_found {
            return Err("Mojang: Palette not found in header".into());
        }
        if !blocks_found {
            return Err("Mojang: Blocks not found in header".into());
        }
        if self.lazy_palette.blocks.is_none() {
            return Err("Mojang: Blocks not initialized after header parsing".into());
        }
        Ok(())
    }

    fn ensure_blocks_initialized(&mut self) {
        if self.lazy_palette.blocks.is_none() {
            let (x, y, z) = self.size;
            self.lazy_palette.blocks = Some(LazyPaletteBlockStoreWrapper::empty_fixed_from_size(x, y, z));
        }
    }

    fn extract_palette_from_nbt_stream(&mut self) -> Result<(), String> {
        let mut palette = HashMap::new();
        let mut current_name = String::new();
        let mut props = HashMap::new();
        let mut depth = 1;

        while depth > 0 {
            match self.parser.next().map_err(|e| e.to_string())? {
                Value::Compound(_) => depth += 1,
                Value::CompoundEnd => {
                    depth -= 1;
                    if depth == 1 {
                        let state = BlockState::from_name_and_properties(&current_name, &props);
                        palette.insert(palette.len() as isize, Rc::new(state));
                        props.clear();
                    }
                }
                Value::String(Some(name), val) => match name.as_str() {
                    "Name" => current_name = val,
                    _ => { props.insert(name, val); }
                }
                Value::ListEnd => break,
                _ => {}
            }
        }

        if let Some(wrapper) = &mut self.lazy_palette.blocks {
            wrapper.set_actual_palette(palette);
        }
        Ok(())
    }

    fn read_blocks_from_nbt_stream(&mut self) -> Result<(), String> {
        let mut coords = [0usize; 3];
        let mut coord_idx = 0;
        let mut depth = 1;

        while depth > 0 {
            match self.parser.next().map_err(|e| e.to_string())? {
                Value::List(_, Tag::Int, 3) => {
                    depth += 1;
                    coord_idx = 0;
                }
                Value::Int(name, val) => match name {
                    Some(ref n) if n == "state" => {
                        if let Some(wrapper) = &mut self.lazy_palette.blocks {
                            wrapper.set_unknown_block_at(coords[0] as i32, coords[1] as i32, coords[2] as i32, val as isize)?;
                        }
                    }
                    None if coord_idx < 3 => {
                        coords[coord_idx] = val as usize;
                        coord_idx += 1;
                    }
                    _ => {}
                },
                Value::ListEnd => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }
}

fn poll_size(reader: &mut Parser<impl std::io::Read>) -> Result<(usize, usize, usize), String> {
    let mut dims = [0usize; 3];
    for i in 0..3 {
        match reader.next().map_err(|e| e.to_string())? {
            Value::Int(_, val) => dims[i] = val as usize,
            _ => return Err("Expected 3 integers for Size".into()),
        }
    }
    Ok((dims[0], dims[1], dims[2]))
}