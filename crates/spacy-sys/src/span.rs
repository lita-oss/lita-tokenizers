use crate::{Error, Tokens};

use pyo3::prelude::*;
use pyo3::types::PyString;

/// A contiguous token span retaining its backing document through a Python reference.
///
/// This handle is valid for its interpreter attachment and does not borrow the language,
/// document handle, or iterator that produced it.
/// Token indices refer to the complete document, even when this span represents one sentence or named entity.
pub struct Span<'py> {
    inner: Bound<'py, PyAny>,
}

impl<'py> Span<'py> {
    pub(crate) fn from_python(inner: Bound<'py, PyAny>) -> Self {
        Self { inner }
    }

    /// Returns the underlying Python span for operations outside the typed interface.
    pub fn as_python(&self) -> &Bound<'py, PyAny> {
        &self.inner
    }

    /// Returns the original span text, excluding whitespace after its final token.
    pub fn text(&self) -> Result<String, Error> {
        self.inner.getattr("text")?.extract()
    }

    /// Returns a Python string handle without extracting a Rust string.
    ///
    /// Accessing spaCy's `Span.text` can still construct a Python string.
    pub fn text_ref(&self) -> Result<Bound<'py, PyString>, Error> {
        Ok(self.inner.getattr("text")?.cast_into()?)
    }

    /// Returns the inclusive zero-based start token index within the document.
    pub fn start(&self) -> Result<usize, Error> {
        self.inner.getattr("start")?.extract()
    }

    /// Returns the exclusive zero-based end token index within the document.
    pub fn end(&self) -> Result<usize, Error> {
        self.inner.getattr("end")?.extract()
    }

    /// Returns the number of tokens in this span, including whitespace tokens.
    pub fn len(&self) -> Result<usize, Error> {
        self.inner.len()
    }

    /// Returns whether this span contains no tokens.
    pub fn is_empty(&self) -> Result<bool, Error> {
        self.len().map(|n| n == 0)
    }

    /// Iterates lazily over this span's tokens in document order.
    ///
    /// Whitespace tokens are included.
    /// The iterator retains its Python reference and terminates after an error or exhaustion,
    /// independently of this span handle's lifetime.
    pub fn tokens(&self) -> Result<Tokens<'py>, Error> {
        Tokens::from_python(&self.inner)
    }

    /// Returns this span's label, or an empty string for an unlabeled span.
    #[cfg(feature = "entities")]
    pub fn label(&self) -> Result<String, Error> {
        self.inner.getattr("label_")?.extract()
    }
}
