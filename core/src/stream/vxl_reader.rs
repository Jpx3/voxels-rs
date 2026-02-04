use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::stream::SchematicInputStream;
use std::cmp::min;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

const MAGIC_NUMBER: i64 = 0x56584C44524D;
const VERSION: i32 = 1;

pub struct VXLSchematicInputStream<R: Read> {
    reader: R,
    palette: HashMap<i32, Arc<BlockState>>,
    header_read: bool,
    axis_order: Option<AxisOrder>,
    boundary: Option<Boundary>,
    read_blocks : usize,
    current_run_state: Option<Arc<BlockState>>,
    remaining_run_length: i32,
}

impl<R: Read> SchematicInputStream for VXLSchematicInputStream<R> {
    fn read(&mut self, buffer: &mut Vec<Block>, offset: usize, length: usize) -> Result<Option<usize>, String> {
        if !self.header_read {
            self.read_header()?;
        }
        if self.boundary.is_none() || self.axis_order.is_none() {
            return Err("VXL: Header not properly read".into());
        }
        let mut blocks_read = 0;
        let mut blocks_written = 0;
        let boundary = self.boundary.unwrap();
        let axis_order = self.axis_order.unwrap();
        while blocks_written < length {
            if self.remaining_run_length <= 0 {
                if !self.parse_next_instruction()? {
                    break;
                }
            }
            let allowed_to_be_written = (length - blocks_written) as i32;
            let attempt_to_write = min(
                allowed_to_be_written,
                self.remaining_run_length
            ) as usize;
            if let Some(state) = &self.current_run_state {
                let mut pos_iter = boundary.iter(axis_order).skip(self.read_blocks);
                for _ in 0..attempt_to_write {
                    let pos = pos_iter.next().ok_or("VXL: Boundary size mismatch (iterator exhausted before stream)")?;
                    if !state.is_air() {
                        let block = Block {
                            position: pos,
                            state: Arc::clone(state),
                        };
                        buffer.push(block);
                        blocks_written += 1;
                    }
                    blocks_read += 1;
                }
            }
            self.remaining_run_length -= attempt_to_write as i32;
        }
        if blocks_read == 0 && length > 0 {
            Ok(None)
        } else {
            Ok(Some(blocks_written))
        }
    }

    fn boundary(&mut self) -> Result<Option<Boundary>, String> {
        if !self.header_read {
            self.read_header()?;
        }
        Ok(self.boundary)
    }
}

impl<R: Read> VXLSchematicInputStream<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader, palette: HashMap::new(),
            header_read: false,
            axis_order: None,
            boundary: None,
            read_blocks: 0,
            current_run_state: None,
            remaining_run_length: 0,
        }
    }

    pub fn read_header(&mut self) -> Result<(Boundary, AxisOrder), String> {
        if self.header_read {
            return Err("VXL: Header already read".into());
        }
        let magic = self.read_var_long()?;
        if magic != MAGIC_NUMBER {
            return Err(format!("VXL: Invalid magic number. Expected 0x{:X}, got 0x{:X}", MAGIC_NUMBER, magic));
        }
        let version = self.read_var_int()?;
        if version != VERSION {
            return Err(format!("VXL: Unsupported version. Expected {}, got {}", VERSION, version));
        }
        let boundary = self.read_boundary()?;
        let axis_order = self.read_axis_order()?;

        self.boundary = Some(boundary);
        self.axis_order = Some(axis_order);

        self.header_read = true;
        Ok((boundary, axis_order))
    }

    fn parse_next_instruction(&mut self) -> Result<bool, String> {
        loop {
            let command_res = self.read_var_int();
            let command = match command_res {
                Ok(c) => c,
                Err(_) => return Ok(false),
            };
            match command {
                0 => {
                    let _ = self.read_var_int()?;
                    let state_str = self.read_string()?;
                    let state = BlockState::from_str(state_str)
                        .map_err(|e| format!("VXL: Parse error: {}", e))?;
                    let id = (self.palette.len() as i32 + 1) * 2;
                    self.palette.insert(id, Arc::new(state));
                }
                1 => {
                    let ref_id = self.read_var_int()?;
                    let diff_str = self.read_string()?;
                    let base = self.palette.get(&ref_id)
                        .ok_or_else(|| format!("VXL: Missing Ref ID {}", ref_id))?;
                    let state = base.update(&diff_str)
                        .map_err(|e| format!("VXL: Diff error: {}", e))?;
                    let id = (self.palette.len() as i32 + 1) * 2;
                    self.palette.insert(id, Arc::new(state));
                }
                cmd => {
                    let is_rle = (cmd & 1) != 0;
                    let id = if is_rle { cmd - 1 } else { cmd };
                    let length = if is_rle { self.read_var_int()? } else { 1 };

                    let state = self.palette.get(&id)
                        .cloned()
                        .ok_or_else(|| format!("VXL: Unknown Palette ID {}", id))?;

                    self.current_run_state = Some(state);
                    self.remaining_run_length = length;
                    return Ok(true);
                }
            }
        }
    }

    fn read_var_int(&mut self) -> Result<i32, String> {
        let mut num = 0;
        let mut shift = 0;
        let mut buf = [0u8; 1];
        loop {
            self.reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
            let byte = buf[0];
            num |= ((byte & 0x7F) as i32) << shift;
            if (byte & 0x80) == 0 { return Ok(num); }
            shift += 7;
            if shift >= 32 { return Err("VXL: VarInt too big".into()); }
        }
    }

    fn read_var_long(&mut self) -> Result<i64, String> {
        let mut num = 0;
        let mut shift = 0;
        let mut buf = [0u8; 1];
        loop {
            self.reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
            let byte = buf[0];
            num |= ((byte & 0x7F) as i64) << shift;
            if (byte & 0x80) == 0 { return Ok(num); }
            shift += 7;
            if shift >= 64 { return Err("VXL: VarLong too big".into()); }
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_var_int()?;
        if len < 0 { return Err("Negative string length".into()); }
        let mut buf = vec![0u8; len as usize];
        self.reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
        String::from_utf8(buf).map_err(|e| e.to_string())
    }

    fn read_boundary(&mut self) -> Result<Boundary, String> {
        let min_x = self.read_var_int()?;
        let min_y = self.read_var_int()?;
        let min_z = self.read_var_int()?;
        let max_x = self.read_var_int()?;
        let max_y = self.read_var_int()?;
        let max_z = self.read_var_int()?;
        Ok(Boundary::new_from_min_max(min_x, min_y, min_z, max_x, max_y, max_z))
    }

    fn read_axis_order(&mut self) -> Result<AxisOrder, String> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
        match buf[0] {
            0 => Ok(AxisOrder::XYZ),
            1 => Ok(AxisOrder::XZY),
            2 => Ok(AxisOrder::YXZ),
            3 => Ok(AxisOrder::YZX),
            4 => Ok(AxisOrder::ZXY),
            5 => Ok(AxisOrder::ZYX),
            n => Err(format!("VXL: Invalid AxisOrder {}", n)),
        }
    }
}