mod pystream;
mod reader;
mod shared;

use pyo3::prelude::*;
use crate::reader::VoxelReader;

#[pymodule]
#[pyo3(name = "voxels_rs")]
fn voxels_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(reader::open, m)?)?;
    m.add_class::<VoxelReader>()?;
    Ok(())
}
