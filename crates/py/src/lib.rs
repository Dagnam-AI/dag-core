//! Python bindings for the Dagnam core. The only crate that names Python.
//!
//! Every binding extracts owned Rust values first, releases the GIL around the call into the
//! pure crate, and builds its result afterwards. No binding formats its input into an error, a
//! log or a panic message: the only text an error carries is a fixed message (plus the number a
//! threshold was refused for), a value of the wrong type is reported by `PyO3` by type name, and
//! a string that is not UTF-8 raises a `UnicodeEncodeError` whose `object` is empty (`PyO3`'s
//! own would hold the whole input, and so would anything that prints or stores the exception).
use pyo3::exceptions::PyUnicodeEncodeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// The text of a Python `str`, owned; a lone surrogate raises a `UnicodeEncodeError` without the text.
fn utf8(text: &Bound<'_, PyString>) -> PyResult<String> {
    text.to_str().map(str::to_owned).map_err(|_| {
        PyUnicodeEncodeError::new_err(("utf-8", "", 0_usize, 0_usize, "surrogates not allowed"))
    })
}

/// The compiled module, imported by `dagnam_core.hygiene`.
#[pymodule]
mod _native {
    use pyo3::exceptions::{PyTypeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::{PyString, PyTuple};

    use super::utf8;

    /// Bumped when any binding changes meaning; the Python caller compares it with the value it was written against.
    #[pymodule_export]
    const CORE_API: u32 = dagnam_hygiene::core_api();

    /// Every PII finding in `text`, sorted by start, as `(start, end, class code)` in code points.
    #[pyfunction]
    fn findings(
        py: Python<'_>,
        text: &Bound<'_, PyString>,
    ) -> PyResult<Vec<(usize, usize, &'static str)>> {
        let text = utf8(text)?;
        Ok(py.detach(move || dagnam_hygiene::findings(&text)))
    }

    /// The 128-value `MinHash` signature of the shingles (any iterable of `str`); every value is
    /// `2**64 - 1` when there are none.
    #[pyfunction]
    fn minhash_signature<'py>(
        py: Python<'py>,
        shingles: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        // Iterating a `str` would hand out its characters: almost certainly a bug, never a signature.
        if shingles.is_instance_of::<PyString>() {
            return Err(PyTypeError::new_err(
                "shingles must be an iterable of str, not a str",
            ));
        }
        let owned = shingles
            .try_iter()?
            .map(|item| utf8(item?.cast::<PyString>()?))
            .collect::<PyResult<Vec<String>>>()?;
        let signature =
            py.detach(move || dagnam_hygiene::minhash_signature(owned.iter().map(String::as_str)));
        PyTuple::new(py, signature)
    }

    /// The indices of every normalised text that duplicates an earlier one, and each verified
    /// `(earlier, later, similarity)` pair; `ValueError` when `threshold` is outside `(0, 1]`.
    #[pyfunction]
    fn near_duplicates(
        py: Python<'_>,
        texts: &Bound<'_, PyAny>,
        threshold: &Bound<'_, PyAny>,
    ) -> PyResult<(Vec<usize>, Vec<dagnam_hygiene::Pair>)> {
        // The reference's own check, on the caller's own object and before any text is read; the
        // message is the f-string's `{threshold}`, so an int reads `0`, not `0.0`.
        let refused = || -> PyResult<PyErr> {
            Ok(PyValueError::new_err(format!(
                "threshold must be in (0, 1]; got {}",
                threshold.str()?
            )))
        };
        if !(threshold.gt(0)? && threshold.le(1)?) {
            return Err(refused()?);
        }
        let value: f64 = threshold.extract()?;
        let owned = texts
            .extract::<Vec<Bound<'_, PyString>>>()?
            .iter()
            .map(utf8)
            .collect::<PyResult<Vec<String>>>()?;
        let clustered = py.detach(move || {
            let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
            dagnam_hygiene::near_duplicates(&borrowed, value)
        });
        // `None` here only when a threshold above 0 in Python rounds to 0.0 as an f64.
        clustered.ok_or_else(|| refused().unwrap_or_else(|e| e))
    }
}
