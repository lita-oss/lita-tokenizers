use crate::{Config, Doc, Docs, Error, Python};

use pyo3::prelude::*;
use pyo3::types::PyDict;

use std::path::Path;

/// An owning handle to a reusable spaCy `Language` pipeline.
///
/// Constructing and configuring a pipeline attaches to the process's Python interpreter.
/// Processing accepts an explicit attachment so callers can inspect results and process multiple inputs
/// without repeatedly entering and leaving Python.
/// Dropping a pipeline does not invalidate documents it has produced.
pub struct Language {
    inner: Py<PyAny>,
}

impl Language {
    pub(crate) fn from_python(inner: Bound<'_, PyAny>) -> Self {
        Self {
            inner: inner.unbind(),
        }
    }

    /// Create a language's default tokenizer without trained components.
    pub fn blank(lang: &str) -> Result<Self, Error> {
        Config::default().blank(lang)
    }

    /// Load an installed pipeline package or a model directory named by a string.
    pub fn load(model: &str) -> Result<Self, Error> {
        Config::default().load(model)
    }

    /// Load an existing model directory without a lossy path conversion.
    pub fn load_path(model: impl AsRef<Path>) -> Result<Self, Error> {
        Config::default().load_path(model)
    }

    /// Return the pipeline's language code.
    pub fn lang(&self) -> Result<String, Error> {
        Python::attach(|py| self.inner.bind(py).getattr("lang")?.extract())
    }

    /// Return active pipeline component names in execution order.
    pub fn pipe_names(&self) -> Result<Vec<String>, Error> {
        Python::attach(|py| self.inner.bind(py).getattr("pipe_names")?.extract())
    }

    /// Return whether a named component is active in the pipeline.
    pub fn has_pipe(&self, name: &str) -> Result<bool, Error> {
        Python::attach(|py| {
            self.inner
                .bind(py)
                .call_method1("has_pipe", (name,))?
                .extract()
        })
    }

    /// Append a component using its registered factory's default configuration.
    ///
    /// Factory names such as `sentencizer` are resolved by spaCy.
    /// Duplicate names and unknown factories return Python exceptions.
    /// Trainable components may need initialization.
    pub fn add_pipe(&mut self, factory: &str) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner.bind(py).call_method1("add_pipe", (factory,))?;
            Ok(())
        })
    }

    /// Remove a named component, discarding spaCy's returned component object.
    pub fn remove_pipe(&mut self, name: &str) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner.bind(py).call_method1("remove_pipe", (name,))?;
            Ok(())
        })
    }

    /// Disable a named component without removing it.
    pub fn disable_pipe(&mut self, name: &str) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner.bind(py).call_method1("disable_pipe", (name,))?;
            Ok(())
        })
    }

    /// Enable a previously disabled component.
    pub fn enable_pipe(&mut self, name: &str) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner.bind(py).call_method1("enable_pipe", (name,))?;
            Ok(())
        })
    }

    /// Tokenize and run all active pipeline components on an input.
    ///
    /// The returned document owns a Python reference and does not borrow the input or pipeline.
    pub fn process<'py>(&self, py: Python<'py>, text: &str) -> Result<Doc<'py>, Error> {
        self.inner.bind(py).call1((text,)).map(Doc::from_python)
    }

    /// Tokenize an input without running the pipeline components.
    pub fn make_doc<'py>(&self, py: Python<'py>, text: &str) -> Result<Doc<'py>, Error> {
        self.inner
            .bind(py)
            .call_method1("make_doc", (text,))
            .map(Doc::from_python)
    }

    /// Process a batch lazily in input order using spaCy's `Language.pipe()`.
    ///
    /// Inputs are converted to a Python sequence at the boundary.
    /// Document processing and result iteration remain lazy.
    /// Batch sizing follows spaCy; no worker processes are requested.
    /// Python exceptions can occur while constructing or advancing the iterator.
    pub fn pipe<'py>(
        &self,
        py: Python<'py>,
        texts: &[&str],
        batch_size: usize,
    ) -> Result<Docs<'py>, Error> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("batch_size", batch_size)?;
        let docs = self
            .inner
            .bind(py)
            .call_method("pipe", (texts,), Some(&kwargs))?;
        Docs::from_python(&docs)
    }

    /// Return the Python pipeline handle for operations outside this wrapper's API.
    ///
    /// Mutations made through Python follow spaCy's own contracts.
    pub fn as_python<'py>(&self, py: Python<'py>) -> &Bound<'py, PyAny> {
        self.inner.bind(py)
    }

    /// Save the pipeline to a directory using spaCy's serialization.
    #[cfg(feature = "serialization")]
    pub fn to_disk(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner
                .bind(py)
                .call_method1("to_disk", (path.as_ref(),))?;
            Ok(())
        })
    }

    /// Restore saved state into this pipeline using spaCy's `Language.from_disk()`.
    ///
    /// The pipeline must already have the appropriate vocabulary and component configuration.
    /// Disabled components remain disabled; this does not rebuild the pipeline's component list.
    /// To construct a pipeline from its saved configuration, use [`Self::load_path()`].
    #[cfg(feature = "serialization")]
    pub fn read_disk(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        Python::attach(|py| {
            self.inner
                .bind(py)
                .call_method1("from_disk", (path.as_ref(),))?;
            Ok(())
        })
    }
}
