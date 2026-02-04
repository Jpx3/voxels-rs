use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::stream::SchematicOutputStream;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MojangSchematicOutputStream<W: std::io::Write> {
    writer: W,
    block: Vec<BlockEntry>,
    palette: Vec<PaletteEntry>,
    palette_map: HashMap<Arc<BlockState>, i32>,
    boundary: Boundary
}

impl<W: std::io::Write> MojangSchematicOutputStream<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            block: Vec::new(),
            palette: Vec::new(),
            palette_map: HashMap::new(),
            boundary: Boundary::new_empty(),
        }
    }
}

#[derive(Serialize)]
struct StructureData {
    #[serde(rename = "DataVersion")]
    data_version: i32,
    #[serde(rename = "size")]
    size: [i32; 3],
    #[serde(rename = "palette")]
    palette: Vec<PaletteEntry>,
    #[serde(rename = "blocks")]
    blocks: Vec<BlockEntry>,
    // #[serde(rename = "entities")]
    // entities: Vec<()>,
}

#[derive(Serialize)]
#[derive(Clone)]
struct PaletteEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Properties", skip_serializing_if = "Option::is_none")]
    properties: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
#[derive(Clone)]
struct BlockEntry {
    #[serde(rename = "pos")]
    pos: [i32; 3],
    #[serde(rename = "state")]
    state: i32,
}

impl<W: std::io::Write> SchematicOutputStream for MojangSchematicOutputStream<W> {
    fn write(&mut self, blocks: &[Block]) -> Result<usize, String> {
        let mut block_count = 0;
        for block in blocks {
            let block_state = block.state.clone();
            let state_index = if let Some(&idx) = self.palette_map.get(block_state.as_ref()) {
                idx
            } else {
                let idx = self.palette.len() as i32;
                let name = block_state.name();
                let props = block_state.properties();
                self.palette.push(PaletteEntry {
                    name,
                    properties: props,
                });
                self.palette_map.insert(block_state, idx);
                idx
            };
            let block_position = block.position;
            self.boundary = self.boundary.expand_to_include(&block_position);
            self.block.push(BlockEntry {
                pos: block_position.to_array(),
                state: state_index,
            });
            block_count += 1;
        }
        Ok(block_count)
    }

    fn complete(&mut self) -> Result<(), String> {
        // fill all missing blocks with air
        let mut full_block_list = Vec::new();
        for pos in self.boundary.iter(AxisOrder::XYZ) {
            if let Some(block_entry) = self.block.iter().find(|b| {
                b.pos == pos.to_array()
            }) {
                full_block_list.push(block_entry.clone());
            } else {
                // air block
                let air_state_index = if let Some(&idx) = self.palette_map.get(BlockState::air_arc().as_ref()) {
                    idx
                } else {
                    let idx = self.palette.len() as i32;
                    self.palette.push(PaletteEntry {
                        name: "minecraft:air".to_string(),
                        properties: None,
                    });
                    self.palette_map.insert(BlockState::air_arc(), idx);
                    idx
                };
                full_block_list.push(BlockEntry {
                    pos: pos.to_array(),
                    state: air_state_index,
                });
            }
        }
        self.block = full_block_list;
        let structure = StructureData {
            data_version: 3465,
            size: self.boundary.size_as_array(),
            palette: self.palette.clone(),
            blocks: self.block.clone()
        };
        let result = match fastnbt::to_writer(&mut self.writer, &structure) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to serialize NBT: {}", e)),
        };
        self.writer.flush().map_err(|e| format!("Failed to flush NBT: {}", e))?;
        result
    }
}
