use std::sync::Arc;
use pyo3::{pyclass, pymethods};
use voxels_core::common::{Block, BlockPosition, BlockState};

#[pyclass]
pub struct PyBlock {
    pub position: BlockPosition,
    pub state: Arc<BlockState>,
}

impl From<Block> for PyBlock {
    fn from(block: Block) -> Self {
        PyBlock {
            position: block.position,
            state: block.state,
        }
    }
}

#[pymethods]
impl PyBlock {
    pub fn position(&self) -> PyBlockPosition {
        self.position.into()
    }

    pub fn state(&self) -> PyBlockState {
        self.state.clone().into()
    }

    pub fn __str__(&self) -> String {
        format!("Block at {} with state {}", self.position().__str__(), self.state().__str__())
    }
}

#[pyclass]
pub struct PyBlockState {
    owning: Arc<BlockState>,
}

#[pymethods]
impl PyBlockState {
    pub fn id(&self) -> String {
        self.owning.name.clone()
    }

    pub fn properties(&self) -> Vec<(String, String)> {
        self.owning.properties.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn __str__(&self) -> String {
        let props = self.owning.properties.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}[{}]", self.owning.name, props)
    }
}

impl From<Arc<BlockState>> for PyBlockState {
    fn from(state: Arc<BlockState>) -> Self {
        PyBlockState {
            owning: state,
        }
    }
}

#[pyclass]
pub struct PyBlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[pymethods]
impl PyBlockPosition {
    pub fn __str__(&self) -> String {
        format!("({}, {}, {})", self.x, self.y, self.z)
    }
    
    pub fn __repr__(&self) -> String {
        self.__str__()
    }
    
    pub fn x(&self) -> i32 {
        self.x
    }
    
    pub fn y(&self) -> i32 {
        self.y
    }
    
    #[getter]
    pub fn z(&self) -> i32 {
        self.z
    }
}


impl From<BlockPosition> for PyBlockPosition {
    fn from(pos: BlockPosition) -> Self {
        PyBlockPosition {
            x: pos.x, y: pos.y, z: pos.z,
        }
    }
}