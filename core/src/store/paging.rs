use crate::common::AxisOrder;

pub trait Page {
    fn load(&self, x: i32, y: i32, z: i32) -> Option<u16>;

    fn store(&mut self, x: i32, y: i32, z: i32, state: u16) -> Result<(), String>;

    fn erase(&mut self, x: i32, y: i32, z: i32) -> Result<(), String>;
}

pub struct ArrayPage {
    size_x: usize, size_y: usize, size_z: usize,
    axis_order : AxisOrder,
    data: Vec<u16>,
    nnz: usize,
}

impl ArrayPage {
    pub(crate) fn new(size_x: usize, size_y: usize, size_z: usize, axis_order: AxisOrder) -> ArrayPage {
        let data = vec![0; size_x * size_y * size_z];
        ArrayPage {
            size_x, size_y, size_z,
            axis_order, data, nnz: 0,
        }
    }

    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let index: i32 = match self.axis_order {
            AxisOrder::XYZ => { x + y * (self.size_x as i32) + z * (self.size_x as i32) * (self.size_y as i32) }
            AxisOrder::XZY => { x + z * (self.size_x as i32) + y * (self.size_x as i32) * (self.size_z as i32) }
            AxisOrder::YXZ => { y + x * (self.size_y as i32) + z * (self.size_y as i32) * (self.size_x as i32) }
            AxisOrder::YZX => { y + z * (self.size_y as i32) + x * (self.size_y as i32) * (self.size_z as i32) }
            AxisOrder::ZXY => { z + x * (self.size_z as i32) + y * (self.size_z as i32) * (self.size_x as i32) }
            AxisOrder::ZYX => { z + y * (self.size_z as i32) + x * (self.size_z as i32) * (self.size_y as i32) }
        };
        if index < 0 || index >= (self.size_x * self.size_y * self.size_z) as i32 {
            None
        } else {
            Some(index as usize)
        }
    }
}

impl Page for ArrayPage {
    fn load(&self, x: i32, y: i32, z: i32) -> Option<u16> {
        match self.data[self.index(x, y, z)?] {
            0 => None,
            state => Some(state - 1),
        }
    }

    fn store(&mut self, x: i32, y: i32, z: i32, state: u16) -> Result<(), String> {
        let idx = self.index(x, y, z)
            .ok_or_else(|| format!("Out of bounds: ({}, {}, {})", x, y, z))?;
        let current = self.data[idx];
        if current == 0 {
            self.nnz += 1;
        }
        self.data[idx] = state + 1;
        Ok(())
    }

    fn erase(&mut self, x: i32, y: i32, z: i32) -> Result<(), String> {
        let idx = self.index(x, y, z)
            .ok_or_else(|| format!("Out of bounds: ({}, {}, {})", x, y, z))?;
        let current = self.data[idx];
        if current != 0 {
            self.nnz -= 1;
            self.data[idx] = 0;
            Ok(())
        } else {
            Err("No block to erase at given coordinates".to_string())
        }
    }
}