use crate::common::AxisOrder;

pub trait Page {
    fn load(&self, x: i32, y: i32, z: i32) -> Option<usize>;

    fn store(&mut self, x: i32, y: i32, z: i32, state: usize) -> Result<(), String>;

    fn erase(&mut self, x: i32, y: i32, z: i32) -> Result<(), String>;

    fn nnz(&self) -> usize;

    fn deep_equals(&self, other: &dyn Page) -> bool {
        if self.nnz() != other.nnz() {
            return false;
        }
        // Note: This is a naive implementation and may not be efficient for large pages.
        for x in 0.. {
            for y in 0.. {
                for z in 0.. {
                    let self_value = self.load(x, y, z);
                    let other_value = other.load(x, y, z);
                    if self_value != other_value {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub struct ArrayPage {
    size_x: usize, size_y: usize, size_z: usize,
    axis_order : AxisOrder,
    data: Vec<usize>,
    nnz: usize,
}

impl ArrayPage {
    pub(crate) fn new(size_x: usize, size_y: usize, size_z: usize, axis_order: AxisOrder) -> Self {
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
    fn load(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        match self.data[self.index(x, y, z)?] {
            0 => None,
            state => Some(state - 1),
        }
    }

    fn store(&mut self, x: i32, y: i32, z: i32, state: usize) -> Result<(), String> {
        let idx = self.index(x, y, z).ok_or("Index out of bounds")?;
        let current = self.data[idx];
        if current == 0 {
            self.nnz += 1;
        }
        self.data[idx] = state + 1;
        Ok(())
    }

    fn erase(&mut self, x: i32, y: i32, z: i32) -> Result<(), String> {
        let idx = self.index(x, y, z).ok_or("Index out of bounds")?;
        let current = self.data[idx];
        if current != 0 {
            self.nnz -= 1;
            self.data[idx] = 0;
            Ok(())
        } else {
            Err("No block to erase at given coordinates".to_string())
        }
    }

    fn nnz(&self) -> usize {
        self.nnz
    }
}