use numpy::PyArray1;
use pyo3::{pyclass, pymethods, Bound, PyResult, Python};
use std::rc::Rc;
use voxels_core::common::{Block, BlockPosition, BlockState, Boundary};

#[pyclass(unsendable)]
pub struct PyBlock {
    pub position: BlockPosition,
    pub state: Rc<BlockState>,
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

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    pub fn __str__(&self) -> String {
        format!("Block at {} with state {}", self.position().__str__(), self.state().__str__())
    }
}

#[pyclass(unsendable)]
pub struct PyBlockState {
    owning: Rc<BlockState>,
}

#[pymethods]
impl PyBlockState {
    pub fn id(&self) -> String {
        self.owning.name()
    }

    pub fn name(&self) -> String {
        self.owning.name()
    }

    pub fn properties(&self) -> Vec<(String, String)> {
        self.owning.properties().iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn __str__(&self) -> String {
        let props = self.owning.properties().iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}[{}]", self.owning.name(), props)
    }
}

impl From<Rc<BlockState>> for PyBlockState {
    fn from(state: Rc<BlockState>) -> Self {
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

    pub fn z(&self) -> i32 {
        self.z
    }
}


impl From<BlockPosition> for PyBlockPosition {
    fn from(pos: BlockPosition) -> Self {
        PyBlockPosition {
            x: pos.x(), y: pos.y(), z: pos.z(),
        }
    }
}

#[pyclass]
pub struct PyBoundary {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub d_x: u32,
    pub d_y: u32,
    pub d_z: u32,
}

#[pymethods]
impl PyBoundary {
    #[new]
    fn new(min_x: i32, min_y: i32, min_z: i32, d_x: u32, d_y: u32, d_z: u32) -> Self {
        PyBoundary { min_x, min_y, min_z, d_x, d_y, d_z }
    }

    pub fn __str__(&self) -> String {
        format!("Boundary(min=({}, {}, {}), size=({}, {}, {}))",
            self.min_x, self.min_y, self.min_z, self.d_x, self.d_y, self.d_z)
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    pub fn min(&self) -> PyBlockPosition {
        PyBlockPosition { x: self.min_x, y: self.min_y, z: self.min_z }
    }

    pub fn max(&self) -> PyBlockPosition {
        PyBlockPosition {
            x: self.min_x + self.d_x as i32 - 1,
            y: self.min_y + self.d_y as i32 - 1,
            z: self.min_z + self.d_z as i32 - 1,
        }
    }

    pub fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let arr = PyArray1::from_vec(py, vec![
            self.min_x as f64, self.min_y as f64, self.min_z as f64,
            self.d_x as f64, self.d_y as f64, self.d_z as f64,
        ]);
        Ok(arr)
    }

    pub fn size(&self) -> (u32, u32, u32) {
        (self.d_x, self.d_y, self.d_z)
    }

    pub fn min_x(&self) -> i32 {
        self.min_x
    }

    pub fn min_y(&self) -> i32 {
        self.min_y
    }

    pub fn min_z(&self) -> i32 {
        self.min_z
    }

    pub fn d_x(&self) -> u32 {
        self.d_x
    }

    pub fn d_y(&self) -> u32 {
        self.d_y
    }

    pub fn d_z(&self) -> u32 {
        self.d_z
    }
}

impl From<Boundary> for PyBoundary {
    fn from(boundary: Boundary) -> Self {
        PyBoundary {
            min_x: boundary.min_x,
            min_y: boundary.min_y,
            min_z: boundary.min_z,
            d_x: boundary.d_x as u32,
            d_y: boundary.d_y as u32,
            d_z: boundary.d_z as u32,
        }
    }
}

impl From<PyBoundary> for Boundary {
    fn from(boundary: PyBoundary) -> Self {
        Boundary {
            min_x: boundary.min_x,
            min_y: boundary.min_y,
            min_z: boundary.min_z,
            d_x: boundary.d_x as i32,
            d_y: boundary.d_y as i32,
            d_z: boundary.d_z as i32,
        }
    }
}