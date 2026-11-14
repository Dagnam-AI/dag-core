//! Python bindings for the Dagnam core. The only crate that names Python.
use pyo3::prelude::*;

/// The compiled module, imported by `dagnam_core.hygiene`.
#[pymodule]
mod _native {
    /// Bumped when any binding changes meaning; the Python side refuses a mismatch.
    #[pymodule_export]
    const CORE_API: u32 = dagnam_hygiene::core_api();
}
