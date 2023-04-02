//! zstd de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::RustyBuffer;
use crate::{AsBytes, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

const DEFAULT_COMPRESSION_LEVEL: i32 = 0;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;
    Ok(())
}

/// ZSTD decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.zstd.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::decompress[data], output_len = output_len).map_err(DecompressionError::from_err)
}

/// ZSTD compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.zstd.compress(b'some bytes here', level=0, output_len=Optional[int])  # level defaults to 11
/// ```
#[pyfunction]
pub fn compress(py: Python, data: BytesType, level: Option<i32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::compress[data], output_len = output_len, level = level)
        .map_err(CompressionError::from_err)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<i32>) -> PyResult<usize> {
    crate::generic!(py, internal::compress[input, output], level = level).map_err(CompressionError::from_err)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    crate::generic!(py, internal::decompress[input, output]).map_err(DecompressionError::from_err)
}

/// ZSTD Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<zstd::stream::write::Encoder<'static, Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(level: Option<i32>) -> PyResult<Self> {
        let inner = zstd::stream::write::Encoder::new(Cursor::new(vec![]), level.unwrap_or(DEFAULT_COMPRESSION_LEVEL))?;
        Ok(Self { inner: Some(inner) })
    }

    /// Compress input into the current compressor's stream.
    pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
        crate::io::stream_compress(&mut self.inner, input)
    }

    /// Flush and return current compressed stream
    pub fn flush(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_flush(&mut self.inner, |e| e.get_mut())
    }

    /// Consume the current compressor state and return the compressed stream
    /// **NB** The compressor will not be usable after this method is called.
    pub fn finish(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|v| v.into_inner()))
    }
}

crate::make_decompressor!();

pub(crate) mod internal {

    use crate::zstd::DEFAULT_COMPRESSION_LEVEL;
    use std::io::{Error, Read, Write};

    /// Decompress gzip data
    #[inline(always)]
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = zstd::stream::read::Decoder::new(input)?;
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    #[inline(always)]
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<i32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL); // 0 will use zstd's default, currently 3
        let mut encoder = zstd::stream::read::Encoder::new(input, level)?;
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
