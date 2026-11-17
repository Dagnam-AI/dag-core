//! Python bindings for the Dagnam core. The only crate that names Python.
//!
//! Every binding extracts owned Rust values first, releases the GIL around the call into the
//! pure crate, and builds its result afterwards. No binding formats its input into an error, a
//! log or a panic message: the only text an error carries is a fixed message (plus the number a
//! threshold was refused for), a value of the wrong type is reported by `PyO3` by type name, and
//! a string that is not UTF-8 raises a `UnicodeEncodeError` whose `object` is empty (`PyO3`'s
//! own would hold the whole input, and so would anything that prints or stores the exception).
//!
//! Python's `\w`, `\d` and `\s` follow the Unicode database of the interpreter, so the module
//! reads `unicodedata.unidata_version` once, when it is imported, and every binding classifies
//! under the tables of that version. An interpreter whose version the core has no tables for
//! (see `dagnam_hygiene::UnicodeVersion`) still imports the module, reports it through
//! `UNICODE_SUPPORTED`, and refuses every binding with a `RuntimeError`.
use std::sync::OnceLock;

use dagnam_hygiene::UnicodeVersion;
use pyo3::exceptions::{PyRuntimeError, PyUnicodeEncodeError};
use pyo3::prelude::*;
use pyo3::types::PyString;

/// What the importing interpreter's Unicode database selected.
enum Selection {
    /// A version the core ships tables for.
    Supported(UnicodeVersion),
    /// Any other version, as the interpreter spells it.
    Unsupported(String),
}

/// Set once, by the module's init.
static SELECTION: OnceLock<Selection> = OnceLock::new();

/// The version to classify under, or a `RuntimeError` that names the interpreter's version (the
/// message never holds any input text).
fn version() -> PyResult<UnicodeVersion> {
    match SELECTION.get() {
        Some(Selection::Supported(version)) => Ok(*version),
        Some(Selection::Unsupported(unidata)) => Err(PyRuntimeError::new_err(format!(
            "dagnam-core has no character tables for Unicode {unidata}"
        ))),
        None => Err(PyRuntimeError::new_err(
            "dagnam-core was not initialised by its module",
        )),
    }
}

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
    use pyo3::types::{PyModule, PyString, PyTuple};

    use super::{SELECTION, Selection, UnicodeVersion, utf8, version};

    /// Bumped when any binding changes meaning; the Python caller compares it with the value it was written against.
    #[pymodule_export]
    const CORE_API: u32 = dagnam_hygiene::core_api();

    /// Reads the interpreter's Unicode version and selects the tables for it.
    #[pymodule_init]
    fn init(module: &Bound<'_, PyModule>) -> PyResult<()> {
        let unidata: String = module
            .py()
            .import("unicodedata")?
            .getattr("unidata_version")?
            .extract()?;
        let selected = UnicodeVersion::from_unidata(&unidata);
        module.add("UNICODE_SUPPORTED", selected.is_some())?;
        module.add("UNICODE_VERSION", unidata.as_str())?;
        // A second init in the same process (a sub-interpreter) reads the same database.
        let _ =
            SELECTION.set(selected.map_or(Selection::Unsupported(unidata), Selection::Supported));
        Ok(())
    }

    /// Every PII finding in `text`, sorted by start, as `(start, end, class code)` in code points.
    #[pyfunction]
    fn findings(
        py: Python<'_>,
        text: &Bound<'_, PyString>,
    ) -> PyResult<Vec<(usize, usize, &'static str)>> {
        let version = version()?;
        let text = utf8(text)?;
        Ok(py.detach(move || dagnam_hygiene::findings(&text, version)))
    }

    /// The 128-value `MinHash` signature of the shingles (any iterable of `str`); every value is
    /// `2**64 - 1` when there are none.
    #[pyfunction]
    fn minhash_signature<'py>(
        py: Python<'py>,
        shingles: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        // Near-duplicate detection reads no character class, but the package is one contract.
        version()?;
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
        version()?;
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
