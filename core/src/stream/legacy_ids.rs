use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use crate::common::BlockState;

const JSON_DATA: &str = include_str!("legacy_ids.json");
const BLOCKS: OnceLock<Arc<HashMap<String, String>>> = OnceLock::new();

pub fn get_blocks() -> Arc<HashMap<String, String>> {
    BLOCKS.get_or_init(|| {
        let map: HashMap<String, String> = serde_json::from_str(JSON_DATA).expect("Failed to parse legacy IDs JSON");
        Arc::new(map)
    }).clone()
}

pub fn convert_legacy_data_to_modern_properties(id: usize, data: u8) -> Option<BlockState> {
    match id {
        // Pistons (Sticky & Normal)
        29 | 33 => {
            let facing = match data & 7 {
                0 => "down",
                1 => "up",
                2 => "north",
                3 => "south",
                4 => "west",
                5 => "east",
                _ => return None,
            };
            let extended = data & 8 != 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("extended".to_string(), extended.to_string()),
            ]))
        }

        // Dispensers & Droppers
        23 | 158 => {
            let facing = match data & 7 {
                0 => "down",
                1 => "up",
                2 => "north",
                3 => "south",
                4 => "west",
                5 => "east",
                _ => return None,
            };
            let triggered = data & 8 != 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("triggered".to_string(), triggered.to_string()),
            ]))
        }

        // Double Slabs
        43 => {
            let type_name = match data & 7 {
                0 => "stone",
                1 => "sandstone",
                2 => "wooden",
                3 => "cobblestone",
                4 => "brick",
                5 => "smooth_stone",
                6 => "nether_brick",
                7 => "quartz",
                _ => "stone",
            };
            Some(BlockState::new(format!("minecraft:double_{}_slab", type_name), vec![]))
        }
        // Slabs
        44 => {
            let half = if data & 8 != 0 { "top" } else { "bottom" };
            let type_name = match data & 7 {
                0 => "stone",
                1 => "sandstone",
                2 => "wooden",
                3 => "cobblestone",
                4 => "brick",
                5 => "smooth_stone",
                6 => "nether_brick",
                7 => "quartz",
                _ => "stone",
            };

            Some(BlockState::new(format!("minecraft:{}_slab", type_name), vec![
                ("half".to_string(), half.to_string()),
            ]))
        }

        // Buttons
        77 | 143 => {
            let facing = match data & 7 {
                0 => "north", // technically "down"
                1 => "east",
                2 => "west",
                3 => "south",
                4 => "north",
                5 => "north", // technically "up"
                _ => "north", // technically "up"
            };
            let face = if data & 7 == 0 { "ceiling" } else if data & 7 == 5 { "floor" } else { "wall" };
            let powered = data & 8 != 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("powered".to_string(), powered.to_string()),
                ("face".to_string(), face.to_string()),
            ]))
        }

        // Stairs
        53 | 67 | 108 | 109 | 114 | 128 | 134 | 135 | 136 | 156 | 163 | 164 | 180 => {
            let facing = match data & 3 {
                0 => "east",
                1 => "west",
                2 => "south",
                3 => "north",
                _ => "north",
            };
            let half = if data & 4 != 0 { "top" } else { "bottom" };
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("half".to_string(), half.to_string()),
                ("shape".to_string(), "straight".to_string()),
            ]))
        }

        // Directional Containers (Chests, Furnaces, Ladders, Wall Signs)
        54 | 61 | 62 | 65 | 68 | 130 => {
            let facing = match data {
                2 => "north",
                3 => "south",
                4 => "west",
                5 => "east",
                _ => "north",
            };
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
            ]))
        }

        // Torches & Redstone Torches
        50 | 75 | 76 => {
            let facing = match data {
                1 => "east",
                2 => "west",
                3 => "south",
                4 => "north",
                5 => "up",
                _ => "up",
            };
            let is_wall = data >= 1 && data <= 4;
            let block_type = match id {
                50 => if is_wall { "minecraft:wall_torch" } else { "minecraft:torch" },
                75 => if is_wall { "minecraft:redstone_wall_torch" } else { "minecraft:redstone_torch" },
                76 => if is_wall { "minecraft:redstone_wall_torch" } else { "minecraft:redstone_torch" },
                _ => return None,
            };
            Some(BlockState::new(block_type.to_string(), vec![
                ("facing".to_string(), facing.to_string()),
                ("lit".to_string(), (id != 75).to_string()),
            ]))
        }

        // Logs (Old & New)
        17 | 162 => {
            let axis = match (data >> 2) & 3 {
                0 => "y",
                1 => "x",
                2 => "z",
                _ => "none",
            };
            // Note: Must pass 'data' here to resolve wood variant (Oak/Birch/etc)
            Some(BlockState::new(get_legacy_type(id, data)?, vec![
                ("axis".to_string(), axis.to_string()),
            ]))
        }

        // Hoppers
        154 => {
            let facing = match data & 7 {
                0 => "down",
                2 => "north",
                3 => "south",
                4 => "west",
                5 => "east",
                _ => "down",
            };
            let enabled = data & 8 == 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("enabled".to_string(), enabled.to_string()),
            ]))
        }

        // Trapdoors
        96 => {
            let facing = match data & 3 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let half = if data & 4 != 0 { "top" } else { "bottom" };
            let open = data & 8 != 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("facing".to_string(), facing.to_string()),
                ("half".to_string(), half.to_string()),
                ("open".to_string(), open.to_string()),
            ]))
        }

        // Fences & Walls
        85 | 139 | 140 | 141 | 142 | 155 => {
            let north = data & 1 != 0;
            let east = data & 2 != 0;
            let south = data & 4 != 0;
            let west = data & 8 != 0;
            Some(BlockState::new(get_legacy_type(id, 0)?, vec![
                ("north".to_string(), north.to_string()),
                ("east".to_string(), east.to_string()),
                ("south".to_string(), south.to_string()),
                ("west".to_string(), west.to_string()),
            ]))
        }

        _ => {
            if let Some(block_type) = get_legacy_type(id, data) {
                Some(BlockState::new(block_type, vec![]))
            } else {
                None
            }
        }
    }
}

pub fn get_legacy_type(id: usize, data: u8) -> Option<String> {
    let key = if data == 0 {
        id.to_string()
    } else {
        format!("{}:{}", id, data)
    };
    get_blocks().get(&key).cloned()
}