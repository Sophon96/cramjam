//! gzip de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::RustyBuffer;
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}

/// Gzip decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.gzip.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
}

/// Gzip compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.gzip.compress(b'some bytes here', level=2, output_len=Optional[int])  # Level defaults to 6
/// ```
#[pyfunction]
pub fn compress(data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(compress(data), output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    let r = internal::compress(input, &mut output, level)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r)
}

pub(crate) mod internal {
    use flate2::read::{GzEncoder, MultiGzDecoder};
    use flate2::Compression;
    use std::io::prelude::*;
    use std::io::{Cursor, Error};

    /// Decompress gzip data
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = MultiGzDecoder::new(input);
        let mut out = vec![];
        let n_bytes = decoder.read_to_end(&mut out)?;
        std::io::copy(&mut Cursor::new(out.as_slice()), output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 6);
        let mut encoder = GzEncoder::new(input, Compression::new(level));
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }

    #[cfg(test)]
    mod tests {

        #[test]
        fn test_gzip_multiple_streams() {
            let mut out1 = vec![];
            let mut out2 = vec![];
            super::compress(b"foo".to_vec().as_slice(), &mut out1, None).unwrap();
            super::compress(b"bar".to_vec().as_slice(), &mut out2, None).unwrap();

            let mut out3 = vec![];
            out1.extend_from_slice(&out2);
            super::decompress(out1.as_slice(), &mut out3).unwrap();
            assert_eq!(out3, b"foobar".to_vec());
        }
    }
}
