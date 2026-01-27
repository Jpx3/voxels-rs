use crate::common::{Block, BlockState};
use crate::store::blockstore::BlockStore;
use crate::stream::SchematicInputStream;
use fastnbt::stream::{Parser, Value};
use std::collections::HashMap;

pub struct SpongeSchematicInputStream<R: std::io::Read> {
    parser: Parser<R>,
}

impl<R: std::io::Read> SpongeSchematicInputStream<R> {
    pub fn new(inner: R) -> Self {
        Self {
            parser: Parser::new(inner),
        }
    }
}

pub fn poll_palette_from_parser<R: std::io::Read>(
    reader: &mut Parser<R>
) -> Result<HashMap<isize, BlockState>, String> {
    let mut result: HashMap<isize, BlockState> = HashMap::new();
    loop {
        match reader.next() {
            Ok(value) => {
                match value {
                    Value::Int(name, val) => {
                        let palette_idx = val as isize;
                        let palette_key = name.ok_or("Expected name for palette entry".to_string())?;
                        result.insert(palette_idx, BlockState::from_str(palette_key)?);
                    },
                    Value::CompoundEnd => {
                        break;
                    },
                    _ => {
                        return Err(format!("Unexpected NBT value: {:?}", value));
                    }
                }
            }
            Err(e) => {
                return Err(format!("Error reading NBT: {}", e));
            }
        }
    }
    Ok(result)
}

impl<R: std::io::Read> SchematicInputStream for SpongeSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut [Block], offset: usize, length: usize) -> Result<Option<usize>, String> {
        let mut indent = 0;

        loop {
            match self.parser.next() {
                Ok(value) => {
                    match value {
                        Value::CompoundEnd => indent -= 2,
                        Value::ListEnd => indent -= 2,
                        _ => {}
                    }
                    match value {
                        Value::ByteArray(_, _) => {
                            println!("Big array read, returning data...");
                        }
                        Value::Compound(ref name) => {
                            if let Some(name) = name {
                                println!("{:indent$}Compound: {}", "", name, indent = indent);
                                if name == "Palette" {
                                    let palette = poll_palette_from_parser(&mut self.parser)?;
                                    for (idx, block_state) in palette.iter() {
                                        println!("{:indent$}Palette idx {}: {:?}", "", idx, block_state, indent = indent + 2);
                                    }
                                }
                            } else {
                                println!("{:indent$}Compound (unnamed)", "", indent = indent);
                            }
                        }
                        _ => {
                            if indent < 16 {
                                // continue;
                                println!("{:indent$}{:?}", "", value, indent = indent);
                            }
                        }
                    }
                    match value {
                        Value::Compound(_) => indent += 2,
                        Value::List(_, _, _) => indent += 2,
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(format!("Error reading NBT: {}", e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_sponge_schematic_input_stream() {
        // let file = File::open("C:\\Users\\strun\\RustroverProjects\\voxels-rs\\test_data\\sponge3-v2.schem").expect("Failed to open schematic file");
        // let reader = BufReader::new(file);
        // let gz_decoder = GzDecoder::new(reader);
        // let mut schematic_stream = SpongeSchematicInputStream::new(gz_decoder);
        // let mut buffer: [Block; 4096] = std::array::from_fn(|_| Block::air());
        // let length = buffer.len();
        // match schematic_stream.read(&mut buffer, 0, length) {
        //     Ok(Some(read_blocks)) => {
        //         println!("Read {} blocks from schematic", read_blocks);
        //     }
        //     Ok(None) => {
        //         println!("End of schematic reached");
        //     }
        //     Err(e) => {
        //         println!("Error reading schematic: {}", e);
        //     }
        // }
    }
}