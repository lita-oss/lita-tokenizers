use crate::{Error, Language, Python};

use pyo3::prelude::*;

use std::path::Path;

/// Import settings for an existing spaCy installation.
///
/// Additional directories are passed to `site.addsitedir()` before spaCy is imported.
/// This processes `.pth` files, including their executable lines.
/// Directories supplement Python's search path and persist after this configuration is dropped.
/// They do not select a Python interpreter, isolate environments, or replace already-imported packages.
/// Configure paths before the first import of spaCy when selecting a custom installation.
///
/// The configuration borrows the caller's path slice; constructing it does not allocate or clone paths.
/// Paths are needed only while creating a pipeline, not while using that pipeline.
#[derive(Debug)]
pub struct Config<'p, P = &'p Path> {
    site_packages: &'p [P],
}

impl<'p> Default for Config<'p> {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl<'p, P> Config<'p, P> {
    /// Borrow existing package directories without allocating or copying paths.
    ///
    /// Relative paths are resolved when a pipeline is created.
    /// Directories are processed in slice order; existing Python search paths retain their precedence.
    /// Elements can be borrowed paths, strings, or owning path types implementing [`AsRef<Path>`](AsRef).
    pub const fn new(site_packages: &'p [P]) -> Self {
        Self { site_packages }
    }
}

impl<P: AsRef<Path>> Config<'_, P> {
    /// Create a language's default tokenizer without trained pipeline components.
    pub fn blank(&self, lang: &str) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("blank", (lang,))?,
            ))
        })
    }

    /// Load an installed pipeline package or a model directory named by a string.
    ///
    /// The package and its dependencies must already be importable.
    /// This does not download it.
    pub fn load(&self, model: &str) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("load", (model,))?,
            ))
        })
    }

    /// Load an existing model directory, preserving non-UTF-8 OS paths.
    ///
    /// Custom factories and other packages required by the model must already be importable.
    pub fn load_path(&self, model: impl AsRef<Path>) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("load", (model.as_ref(),))?,
            ))
        })
    }

    fn import<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        if !self.site_packages.is_empty() {
            let site = py.import("site")?;
            for path in self.site_packages {
                site.call_method1("addsitedir", (path.as_ref().as_os_str(),))?;
            }
        }
        py.import("spacy")
    }
}
