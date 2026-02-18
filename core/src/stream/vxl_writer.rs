use crate::common::{AxisOrder, Block, BlockState, Boundary};
use crate::stream::stream::SchematicOutputStream;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

const MAGIC_NUMBER: i64 = 0x56584C44524D; // "VXLDRM"
const VERSION: i32 = 1;

pub struct VXLSchematicOutputStream<W: Write> {
    writer: W,
    running_palette: HashMap<Rc<BlockState>, i32>,
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

    fn find_closest_state(&self, new_state: &BlockState) -> Option<Rc<BlockState>> {
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
                let air = BlockState::air_rc();
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
        state: &Rc<BlockState>,
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

    fn palette_id_from_state(&mut self, state: &Rc<BlockState>) -> Result<i32, String> {
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
        self.running_palette.insert(Rc::clone(state), new_id);
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


#[cfg(test)]
mod test {
    use crate::common::{AxisOrder, Block, BlockState, Boundary, Region};
    use crate::stream::stream::SchematicOutputStream;
    use std::io::{Cursor, Read};
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn test_vxl_writer() {
        let air_state = BlockState::air_rc();
        let stone_state = Rc::new(BlockState::from_str("minecraft:stone").unwrap());

        let blocks_states = vec![
            None,
            None,
            None,
            Some(stone_state.clone()),
            Some(stone_state.clone()),
            Some(air_state.clone()),
        ];

        let boundary = Boundary::new_from_size(2, 1, 3);
        let blocks: Vec<Block> = boundary.iter(AxisOrder::XYZ)
            .zip(blocks_states.into_iter())
            .filter_map(|(pos, state_opt)| {
                state_opt.map(|state| Block {
                    position: pos,
                    state,
                })
            })
            .collect();

        let mut buffer: Vec<u8> = Vec::new();
        {
            let mut writer = super::VXLSchematicOutputStream::new(
                &mut buffer,
                AxisOrder::XYZ,
                boundary,
            );
            writer.write(&blocks).unwrap();
            writer.complete().unwrap();
        }

        // print to rust !vec for easy copy-paste
        print!("let vxl_data: Vec<u8> = vec![");
        for byte in &buffer {
            print!("{},", byte);
        }
        println!("];");

        assert!(!buffer.is_empty());

        let mut cursor = Cursor::new(&buffer);
        let expected_magic_number:i64 = 0x56584C44524D; // "VXLDRM"
        let magic_number = read_var_long(&mut cursor).unwrap();
        assert_eq!(magic_number, expected_magic_number);
        let expected_version:i32 = 0x01;
        let version = read_var_int(&mut cursor).unwrap();
        assert_eq!(version, expected_version);

        let min_x = read_var_int(&mut cursor).unwrap();
        let min_y = read_var_int(&mut cursor).unwrap();
        let min_z = read_var_int(&mut cursor).unwrap();
        let max_x = read_var_int(&mut cursor).unwrap();
        let max_y = read_var_int(&mut cursor).unwrap();
        let max_z = read_var_int(&mut cursor).unwrap();
        assert_eq!(min_x, 0);
        assert_eq!(min_y, 0);
        assert_eq!(min_z, 0);
        assert_eq!(max_x, 1);
        assert_eq!(max_y, 0);
        assert_eq!(max_z, 2);

        let axis_order_byte = {
            let mut byte = [0u8; 1];
            cursor.read_exact(&mut byte).unwrap();
            byte[0]
        };
        assert_eq!(axis_order_byte, 0);

    //     must be
    //      add air state to the palette as new state (id 2)
    //      push air x3
    //      add stone state to the palette as diff from air (id 4)
    //     push stone x2
    //     push air x1

        let air_command = read_var_int(&mut cursor).unwrap();
        assert_eq!(air_command, 0); // new state
        let _ = read_var_int(&mut cursor).unwrap(); // closest id (0)
        let new_air_str = read_string(&mut cursor).unwrap();
        // can be "minecraft:air" or "minecraft:air[]" depending on implementation
        assert!(new_air_str.starts_with("minecraft:air"));
        let air_push_command = read_var_int(&mut cursor).unwrap();
        assert_eq!(air_push_command, 3); // id 3 for air
        let air_push_length = read_var_int(&mut cursor).unwrap();
        assert_eq!(air_push_length, 3); // run length 3

        let stone_command = read_var_int(&mut cursor).unwrap();
        assert_eq!(stone_command, 1); // diff state
        let closest_id = read_var_int(&mut cursor).unwrap();
        assert_eq!(closest_id, 2); // closest id is air (2)
        let diff_str = read_string(&mut cursor).unwrap();
        assert_eq!(diff_str, ":stone"); // diff from air is just ":stone" (no "minecraft" namespace since it's the same as closest)
        let stone_push_command = read_var_int(&mut cursor).unwrap();
        assert_eq!(stone_push_command, 5); // id 5 for stone with RLE
        let stone_push_length = read_var_int(&mut cursor).unwrap();
        assert_eq!(stone_push_length, 2); // run length 2

        let final_air_command = read_var_int(&mut cursor).unwrap();
        assert_eq!(final_air_command, 2); // id 2 for air without RLE
    }

    fn read_string(reader: &mut dyn Read) -> Result<String, String> {
        let len = read_var_int(reader)?;
        if len < 0 { return Err("Negative string length".into()); }
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
        String::from_utf8(buf).map_err(|e| e.to_string())
    }

    fn read_var_int(reader: &mut dyn Read) -> Result<i32, String> {
        let mut result: i32 = 0;
        let mut shift = 0;
        loop {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte).map_err(|e| e.to_string())?;
            let byte_val = byte[0] as i32;
            result |= (byte_val & 0x7F) << shift;
            if (byte_val & 0x80) == 0 {
                break;
            }
            shift += 7;
        }
        Ok(result)
    }

    fn read_var_long(reader: &mut dyn Read) -> Result<i64, String> {
        let mut result: i64 = 0;
        let mut shift = 0;
        loop {
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte).map_err(|e| e.to_string())?;
            let byte_val = byte[0] as i64;
            result |= (byte_val & 0x7F) << shift;
            if (byte_val & 0x80) == 0 {
                break;
            }
            shift += 7;
        }
        Ok(result)
    }

}