use std::fs::File;
use std::io;
use std::io::{Cursor, Read};
use pyo3::{Bound, Py, PyAny, PyErr, Python};
use pyo3::types::{PyBytes, PyString};
use pyo3::prelude::*;

struct PyStreamAdapter {
    obj: Py<PyAny>,
}

impl Read for PyStreamAdapter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Python::attach(|py| {
            let chunk = self.obj.call_method1(py, "read", (buf.len(),))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            let bytes: &[u8] = chunk.extract(py)
                .map_err(|_e| io::Error::new(io::ErrorKind::Other, "Failed to extract bytes ".to_string()))?;
            let len = bytes.len().min(buf.len());
            buf[..len].copy_from_slice(&bytes[..len]);
            Ok(len)
        })
    }
}

pub fn reader_from(input: &Bound<'_, PyAny>) -> PyResult<Box<dyn Read>> {
    if let Ok(py_str) = input.downcast::<PyString>() {
        let s = py_str.to_str()?;
        if s.starts_with("http") {
            Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>("HTTP not linked"))
        } else {
            Ok(Box::new(File::open(s)?))
        }
    } else if let Ok(py_bytes) = input.downcast::<PyBytes>() {
        Ok(Box::new(Cursor::new(py_bytes.as_bytes().to_vec())))
    } else if input.hasattr("read")? {
        Ok(Box::new(PyStreamAdapter { obj: input.clone().unbind() }))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Input must be a path, URL, bytes, or file-like object"
        ))
    }
}