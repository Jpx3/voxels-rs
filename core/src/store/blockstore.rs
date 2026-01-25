use std::collections::HashMap;
use crate::common::{Axis, AxisOrder, Block, BlockPosition, BlockState, Boundary};
use crate::store::paging::{ArrayPage, Page};

pub trait BlockStore {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<&BlockState>, String>;
    fn set_block_at(&mut self, pos: &BlockPosition, state: &BlockState) -> Result<(), String>;
    fn remove_block_at(&mut self, pos: BlockPosition) -> Result<(), String>;
    fn boundary(&self) -> &Boundary;
    fn set_boundary(&mut self, boundary: Boundary);
    fn resizable(&self) -> bool;

    fn insert(&mut self, blocks: &[Block], offset: usize, length: usize) -> Result<(), String> {
        for i in 0..length {
            let block = &blocks[offset + i];
            self.set_block_at(&block.position, &block.state)?;
        }
        Ok(())
    }
    
    fn _expand_or_throw(&mut self, pos: &BlockPosition) -> Result<(), String> {
        let contains = self.boundary().contains(&pos);
        if !self.resizable() && !contains {
            return Err("Position out of bounds and store is not resizable".to_string());
        } else if !contains {
            self.set_boundary(self.boundary().expand_to_include(&pos));
        }
        Ok(())
    }
}

pub struct SparseBlockStore {
    data: HashMap<BlockPosition, usize>,
    palette: Vec<BlockState>,
    reverse_palette: HashMap<BlockState, usize>,
    boundary: Boundary,
    fixed_size: bool,
}

impl SparseBlockStore {
    pub fn new(boundary: Boundary, fixed_size: bool) -> Self {
        SparseBlockStore {
            data: HashMap::new(),
            palette: Vec::new(),
            reverse_palette: HashMap::new(),
            boundary,
            fixed_size,
        }
    }

    fn get_or_add_palette_index(&mut self, state: &BlockState) -> usize {
        if let Some(&index) = self.reverse_palette.get(state) {
            index
        } else {
            let index = self.palette.len();
            self.palette.push(state.clone());
            self.reverse_palette.insert(state.clone(), index);
            index
        }
    }
}

impl BlockStore for SparseBlockStore {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<&BlockState>, String> {
        if !self.boundary().contains(&pos) {
            return Err("Position out of bounds".to_string());
        }
        match self.data.get(&pos) {
            Some(&index) => Ok(self.palette.get(index)),
            None => Ok(None),
        }
    }

    fn set_block_at(&mut self, pos: &BlockPosition, state: &BlockState) -> Result<(), String> {
        self._expand_or_throw(pos)?;
        let index = self.get_or_add_palette_index(state);
        self.data.insert(pos.clone(), index);
        Ok(())
    }

    fn remove_block_at(&mut self, pos: BlockPosition) -> Result<(), String> {
        self._expand_or_throw(&pos)?;
        self.data.remove(&pos);
        Ok(())
    }

    fn boundary(&self) -> &Boundary {
        &self.boundary
    }

    fn set_boundary(&mut self, boundary: Boundary) {
        self.boundary = boundary;
    }

    fn resizable(&self) -> bool {
        !self.fixed_size
    }
}

pub struct PagedBlockStore {
    pages: HashMap<i64, Box<dyn Page>>,
    palette: Vec<BlockState>,
    reverse_palette: HashMap<BlockState, usize>,
    page_size_x: usize, page_size_y: usize, page_size_z: usize,
    bits_x: u32, bits_y: u32, bits_z: u32,
    mask_x: u32, mask_y: u32, mask_z: u32,
    boundary: Boundary,
    fixed_size: bool,
}

impl PagedBlockStore {
    fn round_to_power_of_two(n: usize) -> usize {
        if n.is_power_of_two() {
            n
        } else {
            n.next_power_of_two()
        }
    }
    
    pub fn empty_resizable() -> Self {
        PagedBlockStore::new(Boundary::new(0, 0, 0, 0, 0, 0), 16, 16, 16, false)
    }

    pub fn from_boundary(boundary: Boundary, fixed_size: bool) -> Self {
        let page_size_x = ((boundary.d_x() / 8) as usize).max(8);
        let page_size_y = ((boundary.d_y() / 8) as usize).max(8);
        let page_size_z = ((boundary.d_z() / 8) as usize).max(8);
        PagedBlockStore::new(boundary, page_size_x, page_size_y, page_size_z, fixed_size)
    }

    pub fn new(boundary: Boundary, page_size_x: usize, page_size_y: usize, page_size_z: usize, fixed_size: bool) -> Self {
        let bits_x = (Self::round_to_power_of_two(page_size_x) as u32).trailing_zeros();
        let bits_y = (Self::round_to_power_of_two(page_size_y) as u32).trailing_zeros();
        let bits_z = (Self::round_to_power_of_two(page_size_z) as u32).trailing_zeros();
        let mask_x = (1u32 << bits_x) - 1;
        let mask_y = (1u32 << bits_y) - 1;
        let mask_z = (1u32 << bits_z) - 1;

        PagedBlockStore {
            pages: HashMap::new(),
            palette: Vec::new(),
            reverse_palette: HashMap::new(),
            page_size_x, page_size_y, page_size_z,
            bits_x, bits_y, bits_z,
            mask_x, mask_y, mask_z,
            boundary, fixed_size,
        }
    }

    fn get_or_add_palette_index(&mut self, state: &BlockState) -> usize {
        if let Some(&index) = self.reverse_palette.get(state) {
            index
        } else {
            let index = self.palette.len();
            self.palette.push(state.clone());
            self.reverse_palette.insert(state.clone(), index);
            index
        }
    }
}

impl BlockStore for PagedBlockStore {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<&BlockState>, String> {
        if !self.boundary().contains(&pos) {
            return Err("Position out of bounds".to_string());
        }
        let page_x = (pos.x as u32) >> self.bits_x;
        let page_y = (pos.y as u32) >> self.bits_y;
        let page_z = (pos.z as u32) >> self.bits_z;
        let page_key = ((page_x as i64) << 40) | ((page_y as i64) << 20) | (page_z as i64);
        if let Some(page) = self.pages.get(&page_key) {
            let local_x = (pos.x as u32) & self.mask_x;
            let local_y = (pos.y as u32) & self.mask_y;
            let local_z = (pos.z as u32) & self.mask_z;
            match page.load(local_x as i32, local_y as i32, local_z as i32) {
                Some(index) => Ok(self.palette.get(index)),
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn set_block_at(&mut self, pos: &BlockPosition, state: &BlockState) -> Result<(), String> {
        self._expand_or_throw(&pos)?;
        let page_x = (pos.x as u32) >> self.bits_x;
        let page_y = (pos.y as u32) >> self.bits_y;
        let page_z = (pos.z as u32) >> self.bits_z;
        let page_key = ((page_x as i64) << 40) | ((page_y as i64) << 20) | (page_z as i64);
        let index = self.get_or_add_palette_index(state);
        let page = self.pages.entry(page_key).or_insert_with(|| {
            Box::new(ArrayPage::new(
                self.page_size_x,
                self.page_size_y,
                self.page_size_z,
                AxisOrder::XYZ,
            ))
        });
        let local_x = (pos.x as u32) & self.mask_x;
        let local_y = (pos.y as u32) & self.mask_y;
        let local_z = (pos.z as u32) & self.mask_z;
        page.store(local_x as i32, local_y as i32, local_z as i32, index)?;
        Ok(())
    }

    fn remove_block_at(&mut self, pos: BlockPosition) -> Result<(), String> {
        self._expand_or_throw(&pos)?;
        let page_x = (pos.x as u32) >> self.bits_x;
        let page_y = (pos.y as u32) >> self.bits_y;
        let page_z = (pos.z as u32) >> self.bits_z;
        let page_key = ((page_x as i64) << 40) | ((page_y as i64) << 20) | (page_z as i64);
        if let Some(page) = self.pages.get_mut(&page_key) {
            let local_x = (pos.x as u32) & self.mask_x;
            let local_y = (pos.y as u32) & self.mask_y;
            let local_z = (pos.z as u32) & self.mask_z;
            page.erase(local_x as i32, local_y as i32, local_z as i32)?;
        }
        Ok(())
    }

    fn boundary(&self) -> &Boundary {
        &self.boundary
    }

    fn set_boundary(&mut self, boundary: Boundary) {
        self.boundary = boundary;
    }

    fn resizable(&self) -> bool {
        !self.fixed_size
    }
}

// testing time
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{BlockPosition, BlockState, Boundary};

    #[test]
    fn test_sparse_block_store() {
        let boundary = Boundary::new(0, 0, 0, 10, 10, 10);
        let mut store = SparseBlockStore::new(boundary, false);
        let pos = BlockPosition { x: 1, y: 1, z: 1 };
        let state = BlockState::from_str("stone".to_string());
        store.set_block_at(&pos, &state).unwrap();
        let retrieved = store.block_at(&pos).unwrap().unwrap();
        assert_eq!(retrieved, &state);
        store.remove_block_at(pos.clone()).unwrap();
        let retrieved = store.block_at(&pos).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_paged_block_store() {
        let boundary = Boundary::new(0, 0, 0, 32, 32, 32);
        let mut store = PagedBlockStore::from_boundary(boundary, true);
        
        let pos = BlockPosition { x: 5, y: 5, z: 5 };
        let state = BlockState::from_str("dirt".to_string());
        
        store.set_block_at(&pos, &state).expect("Failed to set block");
        let retrieved = store.block_at(&pos).unwrap().unwrap();
        assert_eq!(retrieved, &state);
        store.remove_block_at(pos.clone()).unwrap();
        let retrieved = store.block_at(&pos).unwrap();
        assert!(retrieved.is_none());
    }
}