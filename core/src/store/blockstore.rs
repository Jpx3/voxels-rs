use crate::common::{AxisOrder, Block, BlockPosition, BlockState, Boundary, Region};
use crate::store::paging::{ArrayPage, Page};
use std::collections::HashMap;
use std::sync::Arc;
use rustc_hash::FxHashMap;

pub trait BlockStore: Region {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<Arc<BlockState>>, String>;
    fn set_block_at(&mut self, pos: &BlockPosition, state: Arc<BlockState>) -> Result<(), String>;
    fn remove_block_at(&mut self, pos: BlockPosition) -> Result<(), String>;
    fn boundary(&self) -> &Boundary;
    fn set_boundary(&mut self, boundary: Boundary);
    fn resizable(&self) -> bool;

    fn insert(&mut self, blocks: &[Block], offset: usize, length: usize) -> Result<(), String> {
        for i in 0..length {
            let block = &blocks[offset + i];
            self.set_block_at(&block.position, block.state.clone())?;
        }
        Ok(())
    }

    fn iterate_blocks(
        &self,
        axis_order: AxisOrder,
    ) -> Box<dyn Iterator<Item = (BlockPosition, Option<Arc<BlockState>>)> + '_> {
        Box::new(
            self.iter(axis_order)
                .map(move |pos| {
                    let state = self.block_at(&pos).unwrap_or(None);
                    (pos, state)
                })
                .filter(move |(_pos, state)| state.is_some() && !state.as_ref().unwrap().is_air()),
        )
    }

    fn _expand_or_throw(&mut self, pos: &BlockPosition) -> Result<(), String> {
        let contains = self.boundary().contains(&pos);
        if !self.resizable() && !contains {
            return Err("Position out of bounds and store is not resizable".to_string());
        } else if !contains {
            let new_boundary = self.boundary().expand_to_include(&pos);
            if new_boundary.d_x() > 1024 || new_boundary.d_y() > 1024 || new_boundary.d_z() > 1024 {
                return Err("Cannot expand boundary beyond 1024 in any dimension".to_string());
            }
            self.set_boundary(new_boundary);
        }
        Ok(())
    }
}

pub struct SparseBlockStore {
    data: HashMap<BlockPosition, usize>,
    palette: Vec<Arc<BlockState>>,
    reverse_palette: HashMap<Arc<BlockState>, usize>,
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

    fn get_or_add_palette_index(&mut self, state: Arc<BlockState>) -> usize {
        if let Some(&index) = self.reverse_palette.get(&state) {
            index
        } else {
            let index = self.palette.len();
            self.palette.push(state.clone());
            self.reverse_palette.insert(state.clone(), index);
            index
        }
    }
}

impl Region for SparseBlockStore {
    fn contains(&self, pos: &BlockPosition) -> bool {
        self.boundary().contains(pos)
    }

    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_> {
        self.boundary().iter(axis_order)
    }
}

impl BlockStore for SparseBlockStore {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<Arc<BlockState>>, String> {
        if !self.boundary().contains(&pos) {
            return Err("Position out of bounds".to_string());
        }

        match self.data.get(&pos) {
            Some(&index) => Ok(self.palette.get(index).cloned()),
            None => Ok(None),
        }
    }

    fn set_block_at(&mut self, pos: &BlockPosition, state: Arc<BlockState>) -> Result<(), String> {
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
    pages: FxHashMap<i64, Box<dyn Page>>,
    palette: Vec<Arc<BlockState>>,
    reverse_palette: HashMap<Arc<BlockState>, usize>,
    page_size_x: usize,
    page_size_y: usize,
    page_size_z: usize,
    bits_x: u32,
    bits_y: u32,
    bits_z: u32,
    mask_x: u32,
    mask_y: u32,
    mask_z: u32,
    boundary: Boundary,
    fixed_size: bool,
}

impl PagedBlockStore {
    pub fn new_empty_resizable() -> Self {
        PagedBlockStore::new(
            Boundary::new(0, 0, 0, 0, 0, 0),
            16,
            16,
            16,
            false,
        )
    }

    pub fn new_for_fixed_boundary(boundary: Boundary) -> Self {
        PagedBlockStore::new_for_boundary(boundary, true)
    }

    pub fn new_for_boundary(boundary: Boundary, fixed_size: bool) -> Self {
        let page_size_x = ((boundary.d_x() / 8) as usize).max(8);
        let page_size_y = ((boundary.d_y() / 8) as usize).max(8);
        let page_size_z = ((boundary.d_z() / 8) as usize).max(8);
        PagedBlockStore::new(boundary, page_size_x, page_size_y, page_size_z, fixed_size)
    }

    pub fn new(
        boundary: Boundary,
        req_page_size_x: usize,
        req_page_size_y: usize,
        req_page_size_z: usize,
        fixed_size: bool,
    ) -> Self {
        let bits_x = (Self::round_to_power_of_two(req_page_size_x) as u32).trailing_zeros();
        let bits_y = (Self::round_to_power_of_two(req_page_size_y) as u32).trailing_zeros();
        let bits_z = (Self::round_to_power_of_two(req_page_size_z) as u32).trailing_zeros();
        let mask_x = (1u32 << bits_x) - 1;
        let mask_y = (1u32 << bits_y) - 1;
        let mask_z = (1u32 << bits_z) - 1;
        let page_size_x = 1usize << bits_x;
        let page_size_y = 1usize << bits_y;
        let page_size_z = 1usize << bits_z;

        PagedBlockStore {
            pages: FxHashMap::with_capacity_and_hasher(1024, Default::default()),
            palette: Vec::new(),
            reverse_palette: HashMap::new(),
            page_size_x,
            page_size_y,
            page_size_z,
            bits_x,
            bits_y,
            bits_z,
            mask_x,
            mask_y,
            mask_z,
            boundary,
            fixed_size,
        }
    }

    fn get_or_add_palette_index(&mut self, state: Arc<BlockState>) -> usize {
        if let Some(&index) = self.reverse_palette.get(state.as_ref()) {
            index
        } else {
            let index = self.palette.len();
            self.palette.push(state.clone());
            self.reverse_palette.insert(state.clone(), index);
            index
        }
    }

    fn round_to_power_of_two(n: usize) -> usize {
        if n.is_power_of_two() {
            n
        } else {
            n.next_power_of_two()
        }
    }
}

impl Region for PagedBlockStore {
    fn contains(&self, pos: &BlockPosition) -> bool {
        self.boundary().contains(pos)
    }

    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_> {
        Box::new(self.boundary().iter(axis_order))
    }
}

impl BlockStore for PagedBlockStore {
    fn block_at(&self, pos: &BlockPosition) -> Result<Option<Arc<BlockState>>, String> {
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
                Some(index) => Ok(self.palette.get(index).cloned()),
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn set_block_at(&mut self, pos: &BlockPosition, state: Arc<BlockState>) -> Result<(), String> {
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

pub struct LazyPaletteBlockStoreWrapper {
    inner: Box<dyn BlockStore>,
    temp_palette: HashMap<isize, Arc<BlockState>>,
    actual_palette: Option<HashMap<isize, Arc<BlockState>>>,
}

fn temp_state_from_temp_id(
    temp_palette: &mut HashMap<isize, Arc<BlockState>>,
    id: isize,
) -> Arc<BlockState> {
    temp_palette
        .entry(id)
        .or_insert_with(|| {
            Arc::from(BlockState::new(
                "unknown".to_string(),
                vec![("id".to_string(), id.to_string())],
            ))
        })
        .clone()
}

impl Region for LazyPaletteBlockStoreWrapper {
    fn contains(&self, pos: &BlockPosition) -> bool {
        self.inner.contains(pos)
    }

    fn iter(&self, axis_order: AxisOrder) -> Box<dyn Iterator<Item = BlockPosition> + '_> {
        self.inner.iter(axis_order)
    }
}

impl LazyPaletteBlockStoreWrapper {
    pub fn empty_resizable_from_size(size_x: usize, size_y: usize, size_z: usize) -> Self {
        let boundary = Boundary::new(0, 0, 0, size_x as i32, size_y as i32, size_z as i32);
        LazyPaletteBlockStoreWrapper::from(Box::new(PagedBlockStore::new_for_boundary(
            boundary, false,
        )))
    }

    pub fn empty_fixed_from_size(size_x: usize, size_y: usize, size_z: usize) -> Self {
        let boundary = Boundary::new(0, 0, 0, size_x as i32, size_y as i32, size_z as i32);
        LazyPaletteBlockStoreWrapper::from(Box::new(PagedBlockStore::new_for_boundary(boundary, true)))
    }

    pub fn from(inner: Box<dyn BlockStore>) -> Self {
        LazyPaletteBlockStoreWrapper {
            inner,
            temp_palette: HashMap::new(),
            actual_palette: None,
        }
    }

    pub fn block_at(&self, pos: &BlockPosition) -> Result<Option<Arc<BlockState>>, String> {
        match self.actual_palette {
            Some(ref palette) => {
                if let Some(state) = self.inner.block_at(pos)? {
                    let id_str = &state.properties[0].1;
                    let id: isize = id_str
                        .parse()
                        .map_err(|_| "Invalid temporary ID".to_string())?;
                    if let Some(actual_state) = palette.get(&id) {
                        Ok(Some(actual_state.clone()))
                    } else {
                        Err("Temporary ID not found in actual palette".to_string())
                    }
                } else {
                    Ok(None)
                }
            }
            None => Err("Can not access blocks if palette is not provided".to_string()),
        }
    }

    /// Attempt to find the temp id of a state from the actual palette, and if not found, return None
    pub fn state_to_temp_id(&mut self, state: &Arc<BlockState>) -> Option<isize> {
        if let Some(ref palette) = self.actual_palette {
            for (id, actual_state) in palette.iter() {
                if actual_state == state {
                    return Some(*id);
                }
            }
        }
        None
    }

    pub fn set_unknown_block(&mut self, pos: &BlockPosition, id: isize) -> Result<(), String> {
        let state = temp_state_from_temp_id(&mut self.temp_palette, id);
        self.inner.set_block_at(pos, state)
    }

    pub fn set_unknown_block_at(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        state: isize,
    ) -> Result<(), String> {
        let pos = BlockPosition { x, y, z };
        let block_state = temp_state_from_temp_id(&mut self.temp_palette, state);
        self.inner.set_block_at(&pos, block_state)
    }

    pub fn remove_block_at(&mut self, pos: BlockPosition) -> Result<(), String> {
        self.inner.remove_block_at(pos)
    }

    pub fn set_actual_palette(&mut self, palette: HashMap<isize, Arc<BlockState>>) {
        self.actual_palette = Some(palette);
    }
}

// testing time
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{BlockPosition, BlockState, Boundary};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_region_iter_sparse() {
        let boundary = Boundary::new(0, 0, 0, 2, 2, 2);
        let store = SparseBlockStore::new(boundary, false);
        let positions: Vec<BlockPosition> = store.iter(AxisOrder::XYZ).collect();
        let expected_positions = vec![
            BlockPosition { x: 0, y: 0, z: 0 },
            BlockPosition { x: 0, y: 0, z: 1 },
            BlockPosition { x: 0, y: 1, z: 0 },
            BlockPosition { x: 0, y: 1, z: 1 },
            BlockPosition { x: 1, y: 0, z: 0 },
            BlockPosition { x: 1, y: 0, z: 1 },
            BlockPosition { x: 1, y: 1, z: 0 },
            BlockPosition { x: 1, y: 1, z: 1 },
        ];
        assert_eq!(positions, expected_positions);
    }

    #[test]
    fn test_region_iter_different_sizes() {
        let boundary = Boundary::new(0, 0, 0, 3, 2, 1);
        let positions: Vec<BlockPosition> = boundary.iter(AxisOrder::XYZ).skip(2).collect();
        let expected_positions = vec![
            // BlockPosition { x: 0, y: 0, z: 0 },
            // BlockPosition { x: 0, y: 1, z: 0 },
            BlockPosition { x: 1, y: 0, z: 0 },
            BlockPosition { x: 1, y: 1, z: 0 },
            BlockPosition { x: 2, y: 0, z: 0 },
            BlockPosition { x: 2, y: 1, z: 0 },
        ];
        assert_eq!(positions, expected_positions);
    }

    #[test]
    fn test_region_iter_different_sizes_2() {
        let boundary = Boundary::new(0, 0, 0, 1, 3, 2);
        let positions: Vec<BlockPosition> = boundary.iter(AxisOrder::XYZ).skip(3).collect();
        let expected_positions = vec![
            // BlockPosition { x: 0, y: 0, z: 0 },
            // BlockPosition { x: 0, y: 0, z: 1 },
            // BlockPosition { x: 0, y: 1, z: 0 },
            BlockPosition { x: 0, y: 1, z: 1 },
            BlockPosition { x: 0, y: 2, z: 0 },
            BlockPosition { x: 0, y: 2, z: 1 },
        ];
        assert_eq!(positions, expected_positions);
    }

    #[test]
    fn test_sparse_block_store() {
        let boundary = Boundary::new(0, 0, 0, 10, 10, 10);
        let mut store = SparseBlockStore::new(boundary, false);
        let pos = BlockPosition { x: 1, y: 1, z: 1 };
        let state = Arc::from(BlockState::from_string("stone".to_string()).unwrap());
        store.set_block_at(&pos, state.clone()).unwrap();
        let retrieved = store.block_at(&pos).unwrap().unwrap();
        assert_eq!(retrieved, state.clone());
        store.remove_block_at(pos.clone()).unwrap();
        let retrieved = store.block_at(&pos).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_paged_block_store() {
        let boundary = Boundary::new(0, 0, 0, 32, 32, 32);
        let mut store = PagedBlockStore::new_for_boundary(boundary, true);
        let pos = BlockPosition { x: 5, y: 5, z: 5 };
        let state = Arc::from(BlockState::from_string("dirt".to_string()).unwrap());
        store
            .set_block_at(&pos, state.clone())
            .expect("Failed to set block");
        let retrieved = store.block_at(&pos).unwrap().unwrap();
        assert_eq!(retrieved, state.clone());
        store.remove_block_at(pos.clone()).unwrap();
        let retrieved = store.block_at(&pos).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_lazy_palette_block_store() {
        let boundary = Boundary::new(0, 0, 0, 10, 10, 10);
        let inner_store = Box::new(SparseBlockStore::new(boundary, false));
        let mut lazy_store = LazyPaletteBlockStoreWrapper::from(inner_store);
        let pos = BlockPosition { x: 2, y: 2, z: 2 };
        lazy_store.set_unknown_block(&pos, 1).unwrap();
        let mut actual_palette = HashMap::new();
        let block_state = Arc::from(BlockState::from_string("grass".to_string()).unwrap());
        actual_palette.insert(1, block_state.clone());
        lazy_store.set_actual_palette(actual_palette);
        let retrieved = lazy_store.block_at(&pos).unwrap().unwrap();
        assert_eq!(retrieved, block_state.clone());
        lazy_store.remove_block_at(pos.clone()).unwrap();
        let retrieved = lazy_store.block_at(&pos).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_large_page_store() {
        let boundary = Boundary::new(0, 0, 0, 11, 41, 125);
        let mut store = PagedBlockStore::new_for_boundary(boundary, true);
        for x in 0..11 {
            for y in 0..41 {
                for z in 0..125 {
                    let mut rng = ChaCha8Rng::seed_from_u64((x as u64) << 32 | (y as u64) << 16 | (z as u64));
                    let number = rng.next_u32() % 100;
                    let pos = BlockPosition { x, y, z };
                    let state = Arc::from(BlockState::from_string(format!("{}", number)).unwrap());
                    store.set_block_at(&pos, state.clone()).unwrap();
                }
            }
        }
        for x in 0..11 {
            for y in 0..41 {
                for z in 0..125 {
                    let mut rng = ChaCha8Rng::seed_from_u64((x as u64) << 32 | (y as u64) << 16 | (z as u64));
                    let number = rng.next_u32() % 100;
                    let pos = BlockPosition { x, y, z };
                    let expected_state = Arc::from(BlockState::from_string(format!("{}", number)).unwrap());
                    let retrieved_state = store.block_at(&pos).unwrap().unwrap();
                    assert_eq!(retrieved_state, expected_state);
                }
            }
        }
    }
}
