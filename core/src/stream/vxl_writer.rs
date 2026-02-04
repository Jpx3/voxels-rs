use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
use crate::stream::SchematicOutputStream;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

const MAGIC_NUMBER: i64 = 0x56584C44524D; // "VXLDRM"
const VERSION: i32 = 1;

pub struct VXLSchematicOutputStream<W: Write> {
    writer: W,
    running_palette: HashMap<Arc<BlockState>, i32>,
    header_written: bool,
    closed: bool,
    axis_order: AxisOrder,
    boundary: Boundary,
    written_blocks: usize
}

impl<W: Write> SchematicOutputStream for VXLSchematicOutputStream<W> {
    fn write(&mut self, blocks: &[Block]) -> Result<usize, String> {
        if !self.header_written {
            let boundary = Arc::new(self.boundary);
            self.write_header(Arc::clone(&boundary))?;
        }
        self.write_blocks(blocks)
    }

    fn complete(&mut self) -> Result<(), String> {
        self.writer.flush().map_err(|e| e.to_string())?;
        self.closed = true;
        Ok(())
    }
}

impl<W: Write> VXLSchematicOutputStream<W> {
    pub fn new(writer: W, axis_order: AxisOrder, boundary: Boundary) -> Self {
        Self {
            writer,
            running_palette: HashMap::new(),
            header_written: false,
            closed: false,
            axis_order, boundary,
            written_blocks: 0
        }
    }

    pub fn write_header(&mut self, boundary: Arc<Boundary>) -> Result<(), String> {
        if self.header_written {
            return Err("VXL: Header already written".into());
        }
        self.write_var_long(MAGIC_NUMBER);
        self.write_var_int(VERSION);
        self.write_boundary(&boundary)?;
        self.write_axis_order(self.axis_order)?;
        self.header_written = true;
        Ok(())
    }

    fn find_closest_state(&self, new_state: &BlockState) -> Option<Arc<BlockState>> {
        self.running_palette.keys()
            .min_by_key(|state| state.difference(new_state).len())
            .cloned()
    }

    pub fn write_blocks(&mut self, blocks: &[Block]) -> Result<usize, String> {
        if !self.header_written {
            return Err("VXL: Header must be written before blocks".into());
        }
        if self.closed {
            return Err("VXL: Stream is closed".into());
        }
        let mut index = 0;
        let end = blocks.len();
        while index < end {
            let block = &blocks[index];
            let flat_index = self.axis_order.index(
                &block.position,
                &self.boundary
            ) as usize;
            if flat_index < self.written_blocks {
                return Err(format!(
                    "VXL: Blocks out of order. Current cursor at {}, but received block at {}",
                    self.written_blocks, flat_index
                ));
            }
            if flat_index > self.written_blocks {
                let gap = flat_index - self.written_blocks;
                let air = BlockState::air_arc();
                self.write_palette_id_with_rle(&air, gap as i32)?;
                self.written_blocks += gap;
            }
            let mut run_length = 0;
            let start_cursor = self.written_blocks;

            while index + run_length < end {
                let next_block = &blocks[index + run_length];
                if next_block.state != block.state {
                    break;
                }
                let next_flat = self.axis_order.index(&next_block.position, &self.boundary) as usize;
                if next_flat != start_cursor + run_length {
                    break;
                }
                run_length += 1;
            }

            self.write_palette_id_with_rle(&block.state, run_length as i32)?;

            index += run_length;
            self.written_blocks += run_length;
        }

        Ok(self.written_blocks)
    }

    fn write_palette_id_with_rle(
        &mut self,
        state: &Arc<BlockState>,
        run_length: i32,
    ) -> Result<(), String> {
        let palette_id = self.palette_id_from_state(state)?;
        if run_length > 1 {
            self.write_var_int(palette_id + 1);
            self.write_var_int(run_length);
        } else {
            self.write_var_int(palette_id);
        }
        Ok(())
    }

    fn palette_id_from_state(&mut self, state: &Arc<BlockState>) -> Result<i32, String> {
        if let Some(&id) = self.running_palette.get(state) {
            return Ok(id);
        }
        let new_id = (self.running_palette.len() as i32 + 1) * 2;
        if self.running_palette.is_empty() {
            self.write_var_int(0);
            self.write_var_int(0);
            self.write_string(&state.to_string())?;
        } else {
            let closest = self.find_closest_state(state).unwrap();
            let closest_id = *self.running_palette.get(&closest).unwrap();
            let diff_str = closest.difference(state);
            self.write_var_int(1);
            self.write_var_int(closest_id);
            self.write_string(&diff_str)?;
        }
        self.running_palette.insert(Arc::clone(state), new_id);
        Ok(new_id)
    }
}

impl<W: Write> VXLSchematicOutputStream<W> {
    fn write_var_int(&mut self, mut value: i32) {
        let mut buf = [0u8; 5];
        let mut pos = 0;
        loop {
            if (value & !0x7F) == 0 {
                buf[pos] = value as u8;
                self.writer.write_all(&buf[..pos + 1]).expect("Write failed");
                return;
            }
            buf[pos] = ((value & 0x7F) | 0x80) as u8;
            value >>= 7;
            pos += 1;
        }
    }

    fn write_var_long(&mut self, mut value: i64) {
        let mut buf = [0u8; 10];
        let mut pos = 0;
        loop {
            if (value & !0x7F) == 0 {
                buf[pos] = value as u8;
                self.writer.write_all(&buf[..pos + 1]).expect("Write failed");
                return;
            }
            buf[pos] = ((value & 0x7F) | 0x80) as u8;
            value >>= 7;
            pos += 1;
        }
    }

    fn write_string(&mut self, value: &str) -> Result<(), String> {
        let bytes = value.as_bytes();
        self.write_var_int(bytes.len() as i32);
        self.writer.write_all(bytes).map_err(|e| e.to_string())
    }

    fn write_boundary(&mut self, b: &Boundary) -> Result<(), String> {
        self.write_var_int(b.min_x);
        self.write_var_int(b.min_y);
        self.write_var_int(b.min_z);
        self.write_var_int(b.max_x());
        self.write_var_int(b.max_y());
        self.write_var_int(b.max_z());
        Ok(())
    }

    fn write_axis_order(&mut self, order: AxisOrder) -> Result<(), String> {
        let val = match order {
            AxisOrder::XYZ => 0,
            AxisOrder::XZY => 1,
            AxisOrder::YXZ => 2,
            AxisOrder::YZX => 3,
            AxisOrder::ZXY => 4,
            AxisOrder::ZYX => 5,
        };
        self.writer.write_all(&[val]).map_err(|e| e.to_string())
    }
}