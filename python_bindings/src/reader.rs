use voxels_core::stream::stream::SchematicOutputStream;
use crate::pystream::{reader_from, writer_from};
use flate2::bufread::GzDecoder;
use pyo3::prelude::*;
use std::io::{BufReader, BufWriter, Read};
use flate2::write::GzEncoder;
use pyo3::exceptions::{PyRuntimeError, PyStopIteration};
use pyo3::ffi::PyObject;
use pyo3::types::{PyIterator, PyString};
use voxels_core::common::AxisOrder;
use voxels_core::stream::any_reader::AnySchematicInputStream;
use voxels_core::stream::mojang_reader::MojangSchematicInputStream;
use voxels_core::stream::mojang_writer::MojangSchematicOutputStream;
use voxels_core::stream::sponge_reader::SpongeSchematicInputStream;
use voxels_core::stream::sponge_writer::SpongeSchematicOutputStream;
use voxels_core::stream::stream::SchematicInputStream;
use voxels_core::stream::vxl_reader::VXLSchematicInputStream;
use voxels_core::stream::vxl_writer::VXLSchematicOutputStream;
use crate::shared::{PyBlock, PyBoundary};

#[pyclass(unsendable)]
pub struct VoxelReader {
    reader: Option<Box<dyn SchematicInputStream>>,
    entered: bool,
    iterator_called: bool,
}

impl VoxelReader {
    fn new(
        reader: Box<dyn SchematicInputStream>,
    ) -> Self {
        VoxelReader {
            reader: Some(reader),
            entered: false,
            iterator_called: false,
        }
    }
}

#[pymethods]
impl VoxelReader {
    fn __enter__<'py>(slf: Py<Self>, py: Python<'py>) -> PyResult<Py<Self>> {
        let mut ref_mut = slf.borrow_mut(py);
        if ref_mut.entered {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot enter context multiple times"));
        }
        if ref_mut.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is already closed"));
        }
        if ref_mut.iterator_called {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot enter context again after iterating"));
        }
        ref_mut.entered = true;
        Ok(ref_mut.into())
    }

    fn __exit__(
        &mut self,
        _exc_type: &Bound<'_, PyAny>,
        _exc_val: &Bound<'_, PyAny>,
        _exc_tb: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        if !self.entered {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot exit context without entering"));
        }
        if self.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is already closed"));
        }
        self.reader = None;
        Ok(())
    }

    fn __str__(&self) -> PyResult<String> {
        Ok("voxels_rs.read() block".to_string())
    }

    fn boundary(&mut self) -> PyResult<PyBoundary> {
        if !self.entered {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot get boundary without entering context"));
        }
        if self.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"));
        }
        if let Some(reader) = &mut self.reader {
            let result = reader.boundary();
            if let Err(e) = result {
                Err(PyErr::new::<PyRuntimeError, _>(e))
            } else {
                if let Some(boundary) = result.unwrap() {
                    Ok(PyBoundary::from(boundary))
                } else {
                    Err(PyErr::new::<PyRuntimeError, _>("Failed to read boundary"))
                }
            }
        } else {
            Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"))
        }
    }

    fn iter_bulks<'py>(slf: Py<Self>, py: Python<'py>) -> PyResult<Py<Self>> {
        let mut selff = slf.borrow_mut(py);
        if selff.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"));
        }
        if selff.iterator_called {
            return Err(PyErr::new::<PyRuntimeError, _>("Iterator already called"));
        }
        selff.iterator_called = true;
        Ok(selff.into())
    }

    #[inline]
    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Vec<PyBlock>> {
        if !self.entered {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot iterate without entering context"));
        }
        if self.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"));
        }
        if !self.iterator_called {
            return Err(PyErr::new::<PyRuntimeError, _>("Iterator not initialized, call iter_bulks() first"));
        }
        if let Some(reader) = &mut self.reader {
            reader.read_next(1024).map_err(|e| PyErr::new::<PyRuntimeError, _>(e)).and_then(|opt| {
                if let Some(blocks) = opt {
                    Ok(blocks.into_iter().map(|b| {
                        PyBlock::from(b)
                    }).collect())
                } else {
                    Err(PyErr::new::<PyStopIteration, _>("End of stream"))
                }
            })
        } else {
            Err(PyErr::new::<PyStopIteration, _>("Reader is closed"))
        }
    }

    fn read_full(&mut self) -> PyResult<Vec<PyBlock>> {
        if !self.entered {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot read without entering context"));
        }
        if self.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"));
        }
        if self.iterator_called {
            return Err(PyErr::new::<PyRuntimeError, _>("Cannot read full after iterating"));
        }
        if let Some(reader) = &mut self.reader {
            let result = reader.read_to_end_into_vec();
            result.map_err(|e| PyErr::new::<PyRuntimeError, _>(e))
                .map(|blocks| {
                    blocks.into_iter()
                        .map(|b| { PyBlock::from(b) })
                        .collect()
                })
        } else {
            Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"))
        }
    }

    #[pyo3(signature = (path, format="vxl"))]
    fn save(&mut self, path: String, format: &str) -> PyResult<()> {
        if self.reader.is_none() {
            return Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"));
        }
        let stream = BufWriter::new(GzEncoder::new(BufWriter::new(writer_from(path)?), flate2::Compression::default()));
        let boundary = self.boundary()?.into();

        let output_schematic_stream: Box<dyn SchematicOutputStream> = match format.to_ascii_uppercase().as_str() {
            "VXL" => {
                Box::new(VXLSchematicOutputStream::new(stream, AxisOrder::preferred(), boundary))
            },
            "MOJANG" => {
                Box::new(MojangSchematicOutputStream::new(stream))
            },
            "SPONGE" => {
                Box::new(SpongeSchematicOutputStream::new(stream, boundary))
            },
            "AUTO" => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Must specify a concrete type when saving"));
            },
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Unknown format: {}", format))),
        };

        if let Some(reader) = &mut self.reader {
            reader.transfer_into(output_schematic_stream).map_err(|e| PyErr::new::<PyRuntimeError, _>(e))?;
            Ok(())
        } else {
            Err(PyErr::new::<PyRuntimeError, _>("Reader is closed"))
        }
    }

    fn close(&mut self) -> PyResult<()> {
        self.reader = None;
        Ok(())
    }
}

#[pyfunction]
pub fn open(input: &Bound<'_, PyAny>) -> PyResult<VoxelReader> {
    // see if input has a "type" attribute that is of type SchematicType (in python)
    let type_name = input.getattr("type").ok().and_then(|t| {
        if t.is_instance_of::<PyString>() {
            let s = t.downcast::<PyString>().unwrap();
            Some(s.to_str().ok()?.to_string())
        } else if t.hasattr("name").ok()? {
            let name_attr = t.getattr("name").ok()?;
            if name_attr.is_instance_of::<PyString>() {
                let s = name_attr.downcast::<PyString>().unwrap();
                Some(s.to_str().ok()?.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }).unwrap_or_else(|| "auto".to_string()).to_ascii_uppercase();
    let stream = BufReader::new(GzDecoder::new(BufReader::new(reader_from(input)?)));
    match type_name.as_str() {
        "VXL" => {
            Ok(VoxelReader::new(
                Box::new(VXLSchematicInputStream::new(stream)),
            ))
        },
        "MOJANG" => {
            Ok(VoxelReader::new(
                Box::new(MojangSchematicInputStream::new(stream)),
            ))
        },
        "SPONGE" => {
            Ok(VoxelReader::new(
                Box::new(SpongeSchematicInputStream::new(stream)),
            ))
        },
        "AUTO" => {
            Ok(VoxelReader::new(
                Box::new(AnySchematicInputStream::new_from_known(stream)),
            ))
        },
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Unknown type: {}", type_name))),
    }
}
