use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::stream::SchematicOutputStream;
use fastnbt::{ByteArray, IntArray, Value};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use crate::store::blockstore::{BlockStore, PagedBlockStore};

pub struct SpongeSchematicOutputStream<W: Write> {
    writer: W,
    block_store: Box<dyn BlockStore>,
    boundary: Option<Boundary>,
}

impl<W: Write> SpongeSchematicOutputStream<W> {
    pub fn new(writer: W, boundary: Boundary) -> Self {
        SpongeSchematicOutputStream {
            writer,
            block_store: Box::new(PagedBlockStore::new_for_fixed_boundary(boundary)),
            boundary: Some(boundary),
        }
    }

    fn encode_var_int(mut value: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let mut temp = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                temp |= 0x80;
            }
            bytes.push(temp);
            if value == 0 {
                break;
            }
        }
        bytes
    }
}

impl<W: Write> SchematicOutputStream for SpongeSchematicOutputStream<W> {
    fn write(&mut self, blocks: &[Block]) -> Result<usize, String> {
        self.block_store.insert(blocks, 0, blocks.len())?;
        Ok(blocks.len())
    }

    fn complete(&mut self) -> Result<(), String> {
        let boundary = self.boundary.ok_or("Sponge: Boundary must be set before closing")?;
        let mut palette = HashMap::new();
        palette.insert(BlockState::air_arc(), 0);
        let mut block_data_bytes = Vec::new();
        for pos in boundary.iter(AxisOrder::YZX) {
            if !self.block_store.contains(&pos) {
                return Err(format!("Sponge: BlockStore with boundary {:?} is missing position {:?}", boundary, pos));
            }
            let state_index = match self.block_store.block_at(&pos)? {
                None => 0,
                Some(state) => {
                    let palette_size = palette.len() as i32;
                    palette.entry(state.clone())
                        .or_insert_with(|| palette_size)
                        .to_owned()
                }
            };
            block_data_bytes.extend(Self::encode_var_int(state_index));
        }

        let mut palette_nbt = HashMap::new();
        for (block_state, index) in palette {
            palette_nbt.insert(block_state.to_string(), Value::Int(index));
        }

        let mut schematic_compound = HashMap::new();
        schematic_compound.insert("Version".to_string(), Value::Int(3));
        schematic_compound.insert("DataVersion".to_string(), Value::Int(3129));
        schematic_compound.insert("Width".to_string(), Value::Short(boundary.d_x as i16));
        schematic_compound.insert("Height".to_string(), Value::Short(boundary.d_y as i16));
        schematic_compound.insert("Length".to_string(), Value::Short(boundary.d_z as i16));
        schematic_compound.insert("Offset".to_string(), Value::IntArray(IntArray::new(vec![0, 0, 0])));

        let mut blocks_compound = HashMap::new();
        blocks_compound.insert("Palette".to_string(), Value::Compound(palette_nbt));
        let byte_array: Vec<i8> = block_data_bytes.into_iter().map(|b| b as i8).collect();
        blocks_compound.insert("Data".to_string(), Value::ByteArray(ByteArray::new(byte_array)));
        blocks_compound.insert("BlockEntities".to_string(), Value::List(Vec::new()));
        schematic_compound.insert("Blocks".to_string(), Value::Compound(blocks_compound));

        schematic_compound.insert("Metadata".to_string(), Value::Compound({
            let mut meta = HashMap::new();
            meta.insert("Date".to_string(), Value::Long(0));
            meta
        }));

        let mut root = HashMap::new();
        root.insert("Schematic".to_string(), Value::Compound(schematic_compound));
        let nbt_data = Value::Compound(root);
        let encoded = fastnbt::to_bytes(&nbt_data).map_err(|e| format!("Sponge: NBT encoding error: {}", e))?;
        self.writer.write_all(&encoded).map_err(|e| e.to_string())?;
        Ok(())
    }
}