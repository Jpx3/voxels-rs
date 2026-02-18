use crate::common::BlockState;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const JSON_DATA: &str = include_str!("legacy_ids.json");
static BLOCKS: OnceLock<Arc<HashMap<String, String>>> = OnceLock::new();

pub fn get_blocks() -> Arc<HashMap<String, String>> {
    BLOCKS
        .get_or_init(|| {
            let map: HashMap<String, String> =
                serde_json::from_str(JSON_DATA).expect("Failed to parse legacy IDs JSON");
            Arc::new(map)
        })
        .clone()
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
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("extended".to_string(), extended.to_string()),
                ],
            ))
        }

        // Fire
        51 => {
            let age = data & 15;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("age".to_string(), age.to_string())],
            ))
        }

        // Piston Head
        34 => {
            let facing = match data & 7 {
                0 => "down",
                1 => "up",
                2 => "north",
                3 => "south",
                4 => "west",
                5 => "east",
                _ => return None,
            };
            let sticky = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("sticky".to_string(), sticky.to_string()),
                ],
            ))
        }

        // Brewing Stand
        117 => {
            let has_bottle_0 = data & 1 != 0;
            let has_bottle_1 = data & 2 != 0;
            let has_bottle_2 = data & 4 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("has_bottle_0".to_string(), has_bottle_0.to_string()),
                    ("has_bottle_1".to_string(), has_bottle_1.to_string()),
                    ("has_bottle_2".to_string(), has_bottle_2.to_string()),
                ],
            ))
        }

        // Anvil
        145 => {
            let facing = match data & 3 {
                0 => "south",
                1 => "west",
                2 => "north",
                3 => "east",
                _ => "north",
            };

            let damage = (data & 15) >> 2;
            let damage_type_name = match damage & 2 {
                0 => "anvil",
                1 => "chipped_anvil",
                2 => "damaged_anvil",
                _ => "anvil",
            };

            Some(BlockState::new(
                format!("minecraft:{}", damage_type_name),
                vec![("facing".to_string(), facing.to_string())],
            ))
        }

        // Wheat
        59 => {
            let age = data & 7;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("age".to_string(), age.to_string())],
            ))
        }

        // Sunflower
        175 => {
            let half = if data & 8 != 0 { "upper" } else { "lower" };
            let type_name = match data & 7 {
                0 => "sunflower",
                1 => "lilac",
                2 => "tall_grass",
                3 => "large_fern",
                4 => "rose_bush",
                5 => "peony",
                _ => "sunflower",
            };
            Some(BlockState::new(
                format!("minecraft:{}", type_name),
                vec![("half".to_string(), half.to_string())],
            ))
        }

        // Hay Block
        170 => {
            let axis = match data & 12 {
                0 => "y",
                4 => "x",
                8 => "z",
                _ => "y",
            };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("axis".to_string(), axis.to_string())],
            ))
        }

        // Sapling
        6 => {
            let type_name = match data & 7 {
                0 => "oak",
                1 => "spruce",
                2 => "birch",
                3 => "jungle",
                4 => "acacia",
                5 => "dark_oak",
                _ => "oak",
            };
            let stage = (data & 8) >> 3;
            Some(BlockState::new(
                format!("minecraft:{}_sapling", type_name),
                vec![("stage".to_string(), stage.to_string())],
            ))
        }

        // Water & Lava
        8 | 9 | 10 | 11 => {
            let level = data & 15;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("level".to_string(), level.to_string())],
            ))
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
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("triggered".to_string(), triggered.to_string()),
                ],
            ))
        }

        // Doors (Wooden & Iron)
        64 | 71 | 193 | 194 | 195 | 196 | 197 => {
            let half = if data & 8 != 0 { "top" } else { "bottom" };
            let facing = match data & 7 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let open = data & 4 != 0;
            let hinge = if data & 8 != 0 {
                if data & 1 != 0 {
                    "right"
                } else {
                    "left"
                }
            } else {
                "none"
            };
            let powered = if data & 8 != 0 { data & 2 != 0 } else { false };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("half".to_string(), half.to_string()),
                    ("facing".to_string(), facing.to_string()),
                    ("open".to_string(), open.to_string()),
                    ("hinge".to_string(), hinge.to_string()),
                    ("powered".to_string(), powered.to_string()),
                ],
            ))
        }

        // Vines
        106 => {
            let north = data & 1 != 0;
            let east = data & 2 != 0;
            let south = data & 4 != 0;
            let west = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("north".to_string(), north.to_string()),
                    ("east".to_string(), east.to_string()),
                    ("south".to_string(), south.to_string()),
                    ("west".to_string(), west.to_string()),
                ],
            ))
        }

        // Pumpkins & Melons
        86 | 103 => {
            let facing = match data & 7 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("facing".to_string(), facing.to_string())],
            ))
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
            Some(BlockState::new(
                format!("minecraft:double_{}_slab", type_name),
                vec![],
            ))
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

            Some(BlockState::new(
                format!("minecraft:{}_slab", type_name),
                vec![("half".to_string(), half.to_string())],
            ))
        }

        // Wooden Slab
        126 => {
            let type_name = match data & 7 {
                0 => "oak",
                1 => "spruce",
                2 => "birch",
                3 => "jungle",
                4 => "acacia",
                5 => "dark_oak",
                _ => "oak",
            };
            let half = if data & 8 != 0 { "top" } else { "bottom" };
            Some(BlockState::new(
                format!("minecraft:{}_slab", type_name),
                vec![("half".to_string(), half.to_string())],
            ))
        }

        // Sandstone & Purpur Slabs
        182 | 205 => {
            let half = if data & 8 != 0 { "top" } else { "bottom" };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("half".to_string(), half.to_string())],
            ))
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
            let face = if data & 7 == 0 {
                "ceiling"
            } else if data & 7 == 5 {
                "floor"
            } else {
                "wall"
            };
            let powered = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("powered".to_string(), powered.to_string()),
                    ("face".to_string(), face.to_string()),
                ],
            ))
        }

        // Levers
        69 => {
            let facing = match data & 7 {
                0 => "down", // technically "up"
                1 => "east",
                2 => "west",
                3 => "south",
                4 => "north",
                5 => "up", // technically "down"
                _ => "up", // technically "down"
            };
            let face = if data & 7 == 0 {
                "floor"
            } else if data & 7 == 5 {
                "ceiling"
            } else {
                "wall"
            };
            let powered = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("powered".to_string(), powered.to_string()),
                    ("face".to_string(), face.to_string()),
                ],
            ))
        }

        // Beds
        26 => {
            let facing = match data & 3 {
                0 => "south",
                1 => "west",
                2 => "north",
                3 => "east",
                _ => "south",
            };
            let part = if data & 8 != 0 { "head" } else { "foot" };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("part".to_string(), part.to_string()),
                ],
            ))
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
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("half".to_string(), half.to_string()),
                    ("shape".to_string(), "straight".to_string()),
                ],
            ))
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
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("facing".to_string(), facing.to_string())],
            ))
        }

        // Standing Signs
        63 => {
            let facing = match data {
                0 => "south",
                1 => "west",
                2 => "north",
                3 => "east",
                _ => "south",
            };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("facing".to_string(), facing.to_string())],
            ))
        }

        // Banner
        176 | 177 => {
            let facing = match data & 7 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("facing".to_string(), facing.to_string())],
            ))
        }

        // Rails
        66 => {
            let shape = match data & 7 {
                0 => "north_south",
                1 => "east_west",
                2 => "ascending_east",
                3 => "ascending_west",
                4 => "ascending_north",
                5 => "ascending_south",
                6 => "south_east",
                7 => "south_west",
                _ => "north_south",
            };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("shape".to_string(), shape.to_string())],
            ))
        }

        // End Portal Frames
        120 => {
            let facing = match data & 7 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let eye = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("eye".to_string(), eye.to_string()),
                ],
            ))
        }

        // Redstone Wire
        55 => {
            let power = data & 15;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("power".to_string(), power.to_string())],
            ))
        }

        // Repeater
        93 | 94 => {
            let powered = id == 94;
            let facing = match data & 3 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let delay = ((data >> 2) & 3) + 1;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("delay".to_string(), delay.to_string()),
                    ("powered".to_string(), powered.to_string()),
                ],
            ))
        }

        18 => {
            /*
             "18": "minecraft:oak_leaves",
             "18:1": "minecraft:spruce_leaves",
             "18:2": "minecraft:birch_leaves",
             "18:3": "minecraft:jungle_leaves",
             "18:4": "minecraft:acacia_leaves",
             "18:5": "minecraft:dark_oak_leaves",
            */

            let type_name = match (data & 3) % 4 {
                0 => "oak",
                1 => "spruce",
                2 => "birch",
                3 => "jungle",
                4 => "acacia",
                5 => "dark_oak",
                _ => "oak",
            };
            let decayable = data & 4 == 0;
            let check_decay = data & 8 == 0;
            Some(BlockState::new(
                format!("minecraft:{}_leaves", type_name),
                vec![
                    ("decayable".to_string(), decayable.to_string()),
                    ("check_decay".to_string(), check_decay.to_string()),
                ],
            ))
        }

        // Comparator
        149 | 150 => {
            let active = id == 150;
            let facing = match data & 3 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let mode = if data & 8 != 0 { "subtract" } else { "compare" };
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("mode".to_string(), mode.to_string()),
                    ("powered".to_string(), active.to_string()),
                ],
            ))
        }

        // Hopper
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
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("enabled".to_string(), enabled.to_string()),
                ],
            ))
        }

        // Glass Panes & Iron Bars
        102 | 101 | 160 => {
            let north = data & 1 != 0;
            let east = data & 2 != 0;
            let south = data & 4 != 0;
            let west = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("north".to_string(), north.to_string()),
                    ("east".to_string(), east.to_string()),
                    ("south".to_string(), south.to_string()),
                    ("west".to_string(), west.to_string()),
                ],
            ))
        }

        // Cake
        92 => {
            let bites = data & 7;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![("bites".to_string(), bites.to_string())],
            ))
        }

        // Fence
        188 | 189 | 190 | 191 | 192 => {
            let north = data & 1 != 0;
            let east = data & 2 != 0;
            let south = data & 4 != 0;
            let west = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("north".to_string(), north.to_string()),
                    ("east".to_string(), east.to_string()),
                    ("south".to_string(), south.to_string()),
                    ("west".to_string(), west.to_string()),
                ],
            ))
        }

        // Fence Gate
        183 | 184 | 185 | 186 | 187 => {
            let facing = match data & 3 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let open = data & 4 != 0;
            let powered = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("open".to_string(), open.to_string()),
                    ("powered".to_string(), powered.to_string()),
                ],
            ))
        }

        //

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
                50 => {
                    if is_wall {
                        "minecraft:wall_torch"
                    } else {
                        "minecraft:torch"
                    }
                }
                75 => {
                    if is_wall {
                        "minecraft:redstone_wall_torch"
                    } else {
                        "minecraft:redstone_torch"
                    }
                }
                76 => {
                    if is_wall {
                        "minecraft:redstone_wall_torch"
                    } else {
                        "minecraft:redstone_torch"
                    }
                }
                _ => return None,
            };
            Some(BlockState::new(
                block_type.to_string(),
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("lit".to_string(), (id != 75).to_string()),
                ],
            ))
        }
        17 => {
            let axis = match (data >> 2) & 3 {
                0 => "y",
                1 => "x",
                2 => "z",
                _ => "none",
            };
            let type_name = match data & 3 {
                0 => "oak",
                1 => "spruce",
                2 => "birch",
                3 => "jungle",
                4 => "acacia",
                5 => "dark_oak",
                _ => "oak",
            };
            Some(BlockState::new(
                format!("minecraft:{}_log", type_name),
                vec![("axis".to_string(), axis.to_string())],
            ))
        }

        162 => {
            let axis = match (data >> 2) & 3 {
                0 => "y",
                1 => "x",
                2 => "z",
                _ => "none",
            };
            let type_name = match data & 3 {
                0 => "acacia",
                1 => "dark_oak",
                _ => "acacia",
            };
            Some(BlockState::new(
                format!("minecraft:{}_log", type_name),
                vec![("axis".to_string(), axis.to_string())],
            ))
        }

        // Trapdoors
        96 | 107 => {
            let facing = match data & 3 {
                0 => "north",
                1 => "south",
                2 => "west",
                3 => "east",
                _ => "north",
            };
            let half = if data & 4 != 0 { "top" } else { "bottom" };
            let open = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("facing".to_string(), facing.to_string()),
                    ("half".to_string(), half.to_string()),
                    ("open".to_string(), open.to_string()),
                ],
            ))
        }

        // Fences & Walls
        85 | 139 | 140 | 141 | 142 | 155 => {
            let north = data & 1 != 0;
            let east = data & 2 != 0;
            let south = data & 4 != 0;
            let west = data & 8 != 0;
            Some(BlockState::new(
                get_legacy_type(id, 0)?,
                vec![
                    ("north".to_string(), north.to_string()),
                    ("east".to_string(), east.to_string()),
                    ("south".to_string(), south.to_string()),
                    ("west".to_string(), west.to_string()),
                ],
            ))
        }

        _ => {
            if let Some(block_type) = get_legacy_type(id, 0) {
                Some(BlockState::new(block_type, vec![]))
            } else if let Some(block_type) = get_legacy_type(id, data) {
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
