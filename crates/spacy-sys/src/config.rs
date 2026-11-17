use crate::{Error, Language, Python};

use pyo3::prelude::*;

use std::iter::{self, Empty};
use std::path::{Path, PathBuf};

/// Import settings for an existing spaCy installation.
///
/// Constructing a configuration stores its path source without iterating it or initializing Python.
/// Creating or loading a pipeline consumes the configuration and visits its directories in order.
/// Each directory is passed to `site.addsitedir()` before spaCy is imported.
/// This processes `.pth` files, including their executable lines.
/// Directories supplement Python's search path and persist after the configuration is consumed.
/// They do not select a Python interpreter, isolate environments, or replace already-imported packages.
/// Configure paths before the first import of spaCy when selecting a custom installation.
///
/// Sources can own or borrow paths and can generate them lazily.
/// Paths are needed only while creating a pipeline, not while using that pipeline.
#[derive(Debug)]
pub struct Config<I = Empty<PathBuf>> {
    site_packages: I,
}

impl Default for Config {
    fn default() -> Self {
        Self::new(iter::empty())
    }
}

impl<I> Config<I> {
    /// Stores a source of package directories without iterating or collecting it.
    ///
    /// Relative paths are resolved when a pipeline is created.
    /// Processing accepts any [`IntoIterator`] whose items implement [`AsRef<Path>`](AsRef),
    /// including borrowed paths, strings, and owning path types.
    pub const fn new(site_packages: I) -> Self {
        Self { site_packages }
    }
}

impl<I, P> Config<I>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    /// Consumes the configuration and creates a language's tokenizer without trained components.
    pub fn blank(self, lang: &str) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("blank", (lang,))?,
            ))
        })
    }

    /// Consumes the configuration and loads an installed pipeline package or model directory.
    ///
    /// The package and its dependencies must already be importable.
    /// This does not download it.
    pub fn load(self, model: &str) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("load", (model,))?,
            ))
        })
    }

    /// Consumes the configuration and loads a model directory, preserving non-UTF-8 OS paths.
    ///
    /// Custom factories and other packages required by the model must already be importable.
    pub fn load_path(self, model: impl AsRef<Path>) -> Result<Language, Error> {
        Python::attach(|py| {
            Ok(Language::from_python(
                self.import(py)?.call_method1("load", (model.as_ref(),))?,
            ))
        })
    }

    fn import<'py>(self, py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        let mut paths = self.site_packages.into_iter();

        if let Some(path) = paths.next() {
            let site = py.import("site")?;

            for path in iter::once(path).chain(paths) {
                site.call_method1("addsitedir", (path.as_ref().as_os_str(),))?;
            }
        }

        py.import("spacy")
    }
}
