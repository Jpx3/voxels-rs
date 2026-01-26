use std::collections::HashMap;
use fastnbt::stream::{Error, ErrorKind, Parser, Value};
use fastnbt::Tag;
use crate::common::{Block, BlockState, Boundary, Schematic};
use crate::store::blockstore::{BlockStore, LazyPaletteBlockStoreWrapper, PagedBlockStore};
use crate::stream::SchematicInputStream;

pub struct MojangSchematicInputStream<R: std::io::Read> {
    parser: Parser<R>,
    header_read: bool,
    _early_palette: Option<HashMap<isize, BlockState>>,
    _lazy_palette_block_keep: Option<LazyPaletteBlockStoreWrapper>,
}

impl<R: std::io::Read> MojangSchematicInputStream<R> {
    pub fn new(inner: R) -> Self {
        Self {
            parser: Parser::new(inner),
            header_read: false,
            _early_palette: None,
            _lazy_palette_block_keep: None,
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
    fn read(&mut self, buffer: &mut [Block], offset: usize, length: usize) -> Result<Option<usize>, String> {
        if (!self.header_read) {
            self.header_read = true;
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
                                    }
                                } else if let (Some(name), Tag::Compound) = (name, typus) {
                                    if name == "palette" {
                                        // we have the palette, which means we can stream the blocks
                                        // as we are reading the stream
                                    } else if name == "blocks" {
                                        // we have the blocks but not the palette yet, which means we need to
                                        // read all blocks into a temporary buffer first
                                        
                                    }
                                }
                            }
                            _ => {
                                // if indent < 16 {
                                //     // continue;
                                //     println!("{:indent$}{:?}", "", value, indent = indent);
                                // }
                            }
                        }
                        // match value {
                        //     Value::Compound(_) => indent += 2,
                        //     Value::List(_, _, _) => indent += 2,
                        //     _ => {}
                        // }
                    }
                    Err(e) => {
                        return Err(format!("Error reading NBT: {}", e));
                    }
                }
            }
        }
        
        if (self._early_palette.is_some()) {
            // we have the palette, so we can stream blocks directly
        } else if let Some(_lazy_palette_block_keep) = &self._lazy_palette_block_keep {
            // we have the blocks buffered, so we can now build the palette and stream blocks
            
        }
        // 
        
        Ok(None)
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

        let mut buffer: [Block; 4096] = std::array::from_fn(|_| Block::air());
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