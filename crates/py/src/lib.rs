//! Python bindings for the Dagnam core. The only crate that names Python.
//!
//! Every binding extracts owned Rust values first, releases the GIL around the call into the
//! pure crate, and builds its result afterwards. No binding formats its input into an error, a
//! log or a panic message: the only text an error carries is a fixed message (plus the number a
//! threshold was refused for, or the numbers of a limit a batch passed), a value of the wrong type is
//! reported by `PyO3` by type name, and
//! a string that is not UTF-8 raises a `UnicodeEncodeError` whose `object` is empty (`PyO3`'s
//! own would hold the whole input, and so would anything that prints or stores the exception).
//!
//! Python's `\w`, `\d` and `\s` follow the Unicode database of the interpreter, so the module
//! reads `unicodedata.unidata_version` once, when it is imported, and every binding classifies
//! under the tables of that version. An interpreter whose version the core has no tables for
//! (see `dagnam_hygiene::UnicodeVersion`) still imports the module, reports it through
//! `UNICODE_SUPPORTED`, and refuses every binding with a `RuntimeError`.
use std::collections::HashMap;
use std::sync::OnceLock;

use dagnam_hygiene::{Batch, Limits, UnicodeVersion};
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyUnicodeEncodeError, PyValueError};
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
    use dagnam_hygiene::NearDedupError;
    use pyo3::exceptions::{PyTypeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::{PyList, PyModule, PySequence, PyString, PyTuple};

    use super::{
        Batch, CHUNK, Limits, NearDedupLimitError, SELECTION, Selection, Shared, UnicodeVersion,
        utf8, version,
    };

    /// Bumped when any binding changes meaning; the Python caller compares it with the value it was written against.
    #[pymodule_export]
    const CORE_API: u32 = dagnam_hygiene::core_api();

    /// The most texts one `near_duplicates` batch holds.
    #[pymodule_export]
    const MAX_TEXTS: usize = dagnam_hygiene::MAX_TEXTS;

    /// The most characters in one text of a batch.
    #[pymodule_export]
    const MAX_TEXT_CHARS: usize = dagnam_hygiene::MAX_TEXT_CHARS;

    /// The most characters in all the texts of a batch.
    #[pymodule_export]
    const MAX_TOTAL_CHARS: usize = dagnam_hygiene::MAX_TOTAL_CHARS;

    /// The most candidate pairs the walk compares.
    #[pymodule_export]
    const MAX_COMPARISONS: usize = dagnam_hygiene::MAX_COMPARISONS;

    /// The most near-duplicate pairs a batch may have.
    #[pymodule_export]
    const MAX_PAIRS: usize = dagnam_hygiene::MAX_PAIRS;

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
        module.add(
            "NearDedupLimitError",
            module.py().get_type::<NearDedupLimitError>(),
        )?;
        // The first selection stays: a process has one Unicode database, so a later init (the
        // module imported again) keeps the first answer.
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
    /// `(earlier, later, similarity)` pair; `ValueError` when `threshold` is outside `(0, 1]`, and
    /// `NearDedupLimitError` (a `ValueError`) when the batch passes a limit.
    ///
    /// The result is built from shared objects: one `int` per text index and one `float` per
    /// distinct similarity (at most 129), each pair a tuple of three of them, so a pair costs the
    /// tuple and its slot in the list. The pairs are read from the core a chunk at a time and the
    /// chunk is freed once converted, so Rust never holds them all next to the Python list.
    #[pyfunction]
    fn near_duplicates<'py>(
        py: Python<'py>,
        texts: &Bound<'py, PyAny>,
        threshold: &Bound<'py, PyAny>,
    ) -> PyResult<(Bound<'py, PyList>, Bound<'py, PyList>)> {
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
        let limit = |error: NearDedupError| NearDedupLimitError::new_err(error.to_string());
        // The count first, off the sequence itself: a batch over the limit is refused before a
        // pointer of it is collected. (A `str` is a sequence too; the extraction below refuses it.)
        if !texts.is_instance_of::<PyString>()
            && let Ok(sequence) = texts.cast::<PySequence>()
        {
            dagnam_hygiene::check_text_count(sequence.len()?).map_err(limit)?;
        }
        let items = texts.extract::<Vec<Bound<'py, PyString>>>()?;
        // The sizes are read off the `str` objects (a length costs nothing, whatever its size),
        // before any text is copied out of Python.
        dagnam_hygiene::check_text_count(items.len()).map_err(limit)?;
        let lengths = items
            .iter()
            .map(|text| text.len())
            .collect::<PyResult<Vec<usize>>>()?;
        dagnam_hygiene::check_text_lengths(lengths).map_err(limit)?;
        let owned = items.iter().map(utf8).collect::<PyResult<Vec<String>>>()?;
        let walked = py.detach(move || {
            let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
            Batch::new(&borrowed, value, Limits::DEFAULT)
        });
        let mut batch = walked.map_err(|error| match error {
            // Only when a threshold above 0 in Python rounds to 0.0 as an f64.
            NearDedupError::Threshold => refused().unwrap_or_else(|e| e),
            limit_passed => limit(limit_passed),
        })?;
        let mut shared = Shared::new(items.len());
        let pairs = PyList::empty(py);
        let mut chunk = Vec::with_capacity(CHUNK.min(batch.pair_count()));
        for _ in 0..batch.pair_count().div_ceil(CHUNK) {
            chunk.clear();
            py.detach(|| batch.fill(&mut chunk, CHUNK));
            py.check_signals()?;
            for &(low, high, score) in &chunk {
                let pair = [
                    shared.int(py, low)?,
                    shared.int(py, high)?,
                    shared.float(py, score)?,
                ];
                pairs.append(PyTuple::new(py, pair)?)?;
            }
        }
        drop(chunk);
        let duplicates = PyList::empty(py);
        for index in batch.duplicates() {
            duplicates.append(shared.int(py, index)?)?;
        }
        Ok((duplicates, pairs))
    }
}

/// How many pairs are read from the core, and held in Rust, at a time.
const CHUNK: usize = 1 << 16;

/// The objects the pairs of one result share: each text index is one `int`, each similarity one
/// `float`, made when first used.
struct Shared<'py> {
    ints: Vec<Option<Bound<'py, PyAny>>>,
    floats: HashMap<u64, Bound<'py, PyAny>>,
}

impl<'py> Shared<'py> {
    fn new(texts: usize) -> Self {
        Self {
            ints: vec![None; texts],
            floats: HashMap::new(),
        }
    }

    /// The `int` of text `index`.
    fn int(&mut self, py: Python<'py>, index: usize) -> PyResult<Bound<'py, PyAny>> {
        if let Some(Some(made)) = self.ints.get(index) {
            return Ok(made.clone());
        }
        let made = index.into_pyobject(py)?.into_any();
        if let Some(slot) = self.ints.get_mut(index) {
            *slot = Some(made.clone());
        }
        Ok(made)
    }

    /// The `float` of `score`: a similarity is a count of agreeing positions over 128, so a
    /// result holds at most 129 distinct ones.
    fn float(&mut self, py: Python<'py>, score: f64) -> PyResult<Bound<'py, PyAny>> {
        if let Some(made) = self.floats.get(&score.to_bits()) {
            return Ok(made.clone());
        }
        let made = score.into_pyobject(py)?.into_any();
        self.floats.insert(score.to_bits(), made.clone());
        Ok(made)
    }
}

create_exception!(
    dagnam_core.hygiene,
    NearDedupLimitError,
    PyValueError,
    "A `near_duplicates` batch is larger than the core takes: more than `MAX_TEXTS` texts, a text \
     of more than `MAX_TEXT_CHARS` characters, more than `MAX_TOTAL_CHARS` in all, more than \
     `MAX_COMPARISONS` candidate pairs compared or more than `MAX_PAIRS` near-duplicate pairs. The \
     message names the limit and its value and holds no text of the batch."
);
