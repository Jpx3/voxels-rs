use crate::common::{Block, BlockPosition, BlockState, Boundary};
use crate::stream::litematic_bit_array::LitematicaBitArray;
use crate::stream::stream::SchematicInputStream;
use fastnbt::stream::{Parser, Value};
use fastnbt::Tag;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LitematicaSchematicInputStream<R: std::io::Read> {
    parser: Parser<R>,
    /// The currently active region being read
    current_region: Option<LoadedLitematicaRegion>,
    /// Whether we have parsed the header/metadata yet
    header_read: bool,
    /// Helper to track if we have exhausted the file
    finished: bool,
}

struct LoadedLitematicaRegion {
    /// Dimensions of the region (x, y, z)
    size: (usize, usize, usize),
    /// Offset of the region from origin (x, y, z)
    origin: (i32, i32, i32),
    /// The decoded palette
    palette: Vec<Arc<BlockState>>,
    /// The bit array holding block indices. If None, the region is uniform (palette[0]).
    bit_array: Option<LitematicaBitArray>,
    /// Current iteration cursor (x, y, z) local to the region
    cursor: (usize, usize, usize),
}

impl<R: std::io::Read> LitematicaSchematicInputStream<R> {
    pub fn new(inner: R) -> Self {
        Self {
            parser: Parser::new(inner),
            current_region: None,
            header_read: false,
            finished: false,
        }
    }

    fn ensure_region_loaded(&mut self) -> Result<(), String> {
        if self.header_read {
            return Ok(());
        }
        self.header_read = true;
        self.read_litematica_structure()
    }

    /// Main logic to traverse the NBT structure until a valid Region is found and loaded.
    fn read_litematica_structure(&mut self) -> Result<(), String> {
        let mut current_depth = 0;
        let mut regions_depth: Option<usize> = None;

        loop {
            match self.parser.next() {
                Ok(Value::Compound(name_opt)) => {
                    current_depth += 1;
                    if let Some(name) = name_opt {
                        if name == "Regions" {
                            regions_depth = Some(current_depth);
                        }
                    }
                    if let Some(r_depth) = regions_depth {
                        if current_depth == r_depth + 1 {
                            let region = self.parse_region_compound()?;
                            self.current_region = Some(region);
                            return Ok(());
                        }
                    }
                }
                Ok(Value::CompoundEnd) => {
                    if let Some(r_depth) = regions_depth {
                        if current_depth == r_depth {
                            self.finished = true;
                            return Ok(());
                        }
                    }
                    current_depth -= 1;
                    if current_depth < 0 {
                        return Err("NBT Structure Error: Negative depth".into());
                    }
                }
                Err(e) if e.is_eof() => {
                    self.finished = true;
                    return Ok(());
                }
                Err(e) => return Err(format!("NBT Stream Error: {}", e)),
                Ok(_) => {}
            }
        }
    }

    fn parse_region_compound(&mut self) -> Result<LoadedLitematicaRegion, String> {
        let mut size = (0, 0, 0);
        let mut origin = (0, 0, 0);
        let mut palette = Vec::new();
        let mut block_states_data: Option<Vec<i64>> = None;
        let mut depth = 1;

        while depth > 0 {
            match self.parser.next().map_err(|e| e.to_string())? {
                // --- Position ---
                Value::Compound(Some(name)) if name == "Position" => {
                    origin = self.read_xyz_compound()?;
                    // read_xyz_compound consumes the end tag, so we are back at depth
                }

                // --- Size ---
                Value::Compound(Some(name)) if name == "Size" => {
                    let s = self.read_xyz_compound()?;
                    // Litematica sizes can be negative, implying direction.
                    // We take absolute size for storage, origin logic handles position.
                    size = (s.0.abs() as usize, s.1.abs() as usize, s.2.abs() as usize);
                }

                Value::Compound(_) => depth += 1,
                Value::CompoundEnd => depth -= 1,

                // --- BlockStatePalette ---
                Value::List(Some(name), Tag::Compound, _) if name == "BlockStatePalette" => {
                    palette = self.read_palette_list()?;
                }

                // --- BlockStates (The heavy data) ---
                Value::LongArray(Some(name), data) if name == "BlockStates" => {
                    block_states_data = Some(data);
                }

                _ => {}
            }
        }

        if size == (0, 0, 0) {
            return Err("Invalid Litematica region: Size is 0".into());
        }
        if palette.is_empty() {
            // Even air-only regions usually have air in palette
            return Err("Invalid Litematica region: Empty Palette".into());
        }

        // Construct BitArray
        let bit_array = if let Some(data) = block_states_data {
            let total_blocks = size.0 * size.1 * size.2;

            // Litematica nbits calculation: max(2, ceil(log2(palette_len)))
            let mut nbits = 2;
            if palette.len() > 1 {
                let p = palette.len() - 1; // 0-indexed max value
                let width = (usize::BITS - p.leading_zeros()) as usize;
                nbits = std::cmp::max(2, width);
            }

            Some(LitematicaBitArray::from_nbt(data, total_blocks, nbits)?)
        } else {
            // Uniform region (e.g., all Air or all Stone)
            None
        };

        Ok(LoadedLitematicaRegion {
            size,
            origin,
            palette,
            bit_array,
            cursor: (0, 0, 0),
        })
    }

    /// Reads a generic XYZ compound (used for Size and Position)
    fn read_xyz_compound(&mut self) -> Result<(i32, i32, i32), String> {
        let mut vec = (0, 0, 0);
        let mut depth = 1;
        while depth > 0 {
            match self.parser.next().map_err(|e| e.to_string())? {
                Value::Int(Some(name), val) => match name.as_str() {
                    "x" => vec.0 = val,
                    "y" => vec.1 = val,
                    "z" => vec.2 = val,
                    _ => {}
                },
                Value::Compound(_) => depth += 1, // Should not happen in pure xyz struct
                Value::CompoundEnd => depth -= 1,
                _ => {}
            }
        }
        Ok(vec)
    }

    fn read_palette_list(&mut self) -> Result<Vec<Arc<BlockState>>, String> {
        let mut palette = Vec::new();
        // The parser is currently at the List Start.
        // We iterate until ListEnd.
        // Each entry is a Compound representing a BlockState.

        let mut current_name = String::new();
        let mut props = HashMap::new();
        let mut depth = 1; // inside the list

        while depth > 0 {
            match self.parser.next().map_err(|e| e.to_string())? {
                Value::Compound(None) => {
                    // Start of a palette entry
                    depth += 1;
                    current_name.clear();
                    props.clear();
                },

                // Properties map
                Value::Compound(Some(name)) if name == "Properties" => {
                    // Read properties kv pairs
                    let mut p_depth = 1;
                    while p_depth > 0 {
                        match self.parser.next().map_err(|e| e.to_string())? {
                            Value::String(Some(k), v) => { props.insert(k, v); }
                            Value::Compound(_) => p_depth += 1,
                            Value::CompoundEnd => p_depth -= 1,
                            _ => {}
                        }
                    }
                },

                Value::String(Some(name), val) if name == "Name" => {
                    current_name = val;
                },

                Value::CompoundEnd => {
                    depth -= 1;
                    if depth == 1 {
                        // End of one palette entry compound
                        // depth 1 means we are back in the List
                        let state = BlockState::from_name_and_properties(&current_name, &props);
                        palette.push(Arc::new(state));
                    }
                },

                Value::ListEnd => break, // End of BlockStatePalette list
                _ => {}
            }
        }
        Ok(palette)
    }
}

impl<R: std::io::Read> SchematicInputStream for LitematicaSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, _offset: usize, length: usize) -> Result<Option<usize>, String> {
        self.ensure_region_loaded()?;

        let region = match &mut self.current_region {
            Some(r) => r,
            None => return Ok(None),
        };

        let (sx, sy, sz) = region.size;
        let mut written = 0;

        // Iterate until we fill length or finish region
        while written < length {
            let (x, y, z) = region.cursor;
            if y >= sy {
                self.current_region = None;
                break;
            }
            let index = (y * sz + z) * sx + x;
            let state = if let Some(bits) = &region.bit_array {
                let palette_idx = bits.get(index).unwrap_or(0) as usize;
                region.palette.get(palette_idx).cloned().unwrap_or_else(|| Arc::new(BlockState::air()))
            } else {
                region.palette.get(0).cloned().unwrap_or_else(|| Arc::new(BlockState::air()))
            };

            if !state.is_air() {
                let abs_x = region.origin.0 + x as i32;
                let abs_y = region.origin.1 + y as i32;
                let abs_z = region.origin.2 + z as i32;
                buffer.push(Block::new(
                    state,
                    BlockPosition::new(abs_x, abs_y, abs_z )
                ));
                written += 1;
            }

            region.cursor.0 += 1;
            if region.cursor.0 >= sx {
                region.cursor.0 = 0;
                region.cursor.2 += 1;
                if region.cursor.2 >= sz {
                    region.cursor.2 = 0;
                    region.cursor.1 += 1;
                }
            }
        }

        if written == 0 && self.current_region.is_none() {
            return Ok(None);
        }

        Ok(Some(written))
    }

    fn boundary(&mut self) -> Result<Option<Boundary>, String> {
        self.ensure_region_loaded()?;
        match &self.current_region {
            Some(r) => {
                let (w, h, d) = r.size;
                Ok(Some(Boundary::new(
                    r.origin.0, r.origin.1, r.origin.2,
                    r.origin.0 + w as i32,
                    r.origin.1 + h as i32,
                    r.origin.2 + d as i32
                )))
            },
            None => Ok(None),
        }
    }
}