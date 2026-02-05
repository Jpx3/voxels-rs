pub struct LitematicaBitArray {
    size: usize,
    nbits: usize,
    array: Vec<u64>,
    mask: u64,
}

impl LitematicaBitArray {
    pub fn new(size: usize, nbits: usize) -> Self {
        let s = ((nbits * size) as f64 / 64.0).ceil() as usize;
        Self {
            size,
            nbits,
            array: vec![0; s],
            mask: (1u64 << nbits) - 1,
        }
    }

    pub fn from_nbt(arr: Vec<i64>, size: usize, nbits: usize) -> Result<Self, String> {
        let expected_len = ((size * nbits) as f64 / 64.0).ceil() as usize;
        if expected_len != arr.len() {
            return Err(format!("Length mismatch: expected {}, got {}", expected_len, arr.len()));
        }
        let u_array = arr.into_iter().map(|x| x as u64).collect();
        Ok(Self {
            size,
            nbits,
            array: u_array,
            mask: (1u64 << nbits) - 1,
        })
    }

    pub fn to_nbt_vec(&self) -> Vec<i64> {
        self.array.iter().map(|&x| x as i64).collect()
    }

    pub fn get(&self, index: usize) -> Option<u64> {
        if index >= self.size {
            return None;
        }
        let start_offset = index * self.nbits;
        let start_arr_index = start_offset >> 6;
        let end_arr_index = ((index + 1) * self.nbits - 1) >> 6;
        let start_bit_offset = start_offset & 0x3F;
        if start_arr_index == end_arr_index {
            Some((self.array[start_arr_index] >> start_bit_offset) & self.mask)
        } else {
            let end_offset = 64 - start_bit_offset;
            let val = (self.array[start_arr_index] >> start_bit_offset) | (self.array[end_arr_index] << end_offset);
            Some(val & self.mask)
        }
    }

    pub fn set(&mut self, index: usize, value: u64) -> Result<(), String> {
        if index >= self.size {
            return Err("Index out of bounds".into());
        }
        if value > self.mask {
            return Err(format!("Value {} exceeds mask {}", value, self.mask));
        }
        let start_offset = index * self.nbits;
        let start_arr_index = start_offset >> 6;
        let end_arr_index = ((index + 1) * self.nbits - 1) >> 6;
        let start_bit_offset = start_offset & 0x3F;
        self.array[start_arr_index] &= !(self.mask << start_bit_offset);
        self.array[start_arr_index] |= value << start_bit_offset;
        if start_arr_index != end_arr_index {
            let end_offset = 64 - start_bit_offset;
            let j1 = self.nbits - end_offset;
            self.array[end_arr_index] = (self.array[end_arr_index] >> j1 << j1) | (value >> end_offset);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.size
    }
}