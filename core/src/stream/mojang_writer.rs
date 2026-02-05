use crate::common::{AxisOrder, Block, BlockPosition, BlockState, Boundary, Region};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use crate::stream::stream::SchematicOutputStream;

pub struct MojangSchematicOutputStream<W: std::io::Write> {
    writer: W,
    block: HashMap<BlockPosition, BlockEntry>,
    palette: Vec<PaletteEntry>,
    palette_map: HashMap<Arc<BlockState>, i32>,
    boundary: Boundary
}

impl<W: std::io::Write> MojangSchematicOutputStream<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            block: HashMap::new(),
            palette: Vec::new(),
            palette_map: HashMap::new(),
            boundary: Boundary::new_empty(),
        }
    }

    fn palette_idx_from_state(&mut self, state: &Arc<BlockState>) -> i32 {
        if let Some(&idx) = self.palette_map.get(state) {
            idx
        } else {
            let idx = self.palette.len() as i32;
            let name = state.name();
            let props = state.properties();
            self.palette.push(PaletteEntry {
                name,
                properties: props,
            });
            self.palette_map.insert(Arc::clone(state), idx);
            idx
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
            let state_index = self.palette_idx_from_state(&block_state);
            let block_position = block.position;
            self.boundary = self.boundary.expand_to_include(&block_position);
            self.block.insert(block_position, BlockEntry {
                pos: block_position.to_array(),
                state: state_index,
            });
            block_count += 1;
        }
        Ok(block_count)
    }

    fn complete(&mut self) -> Result<(), String> {
        let air_state_index = self.palette_idx_from_state(&BlockState::air_arc());
        let mut full_block_list = Vec::new();
        for pos in self.boundary.iter(AxisOrder::XYZ) {
            let block_entry = self.block.get(&pos);
            if let Some(entry) = block_entry {
                full_block_list.push(entry.clone());
            } else {
                full_block_list.push(BlockEntry {
                    pos: pos.to_array(),
                    state: air_state_index,
                });
            }
        }
        let structure = StructureData {
            data_version: 3465,
            size: self.boundary.size_as_array(),
            palette: self.palette.clone(),
            blocks: full_block_list,
        };
        let result = match fastnbt::to_writer(&mut self.writer, &structure) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to serialize NBT: {}", e)),
        };
        self.writer.flush().map_err(|e| format!("Failed to flush NBT: {}", e))?;
        result
    }
}
