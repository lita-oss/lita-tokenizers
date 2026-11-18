use crate::{Error, Spans, Token, Tokens};

use pyo3::prelude::*;
#[cfg(feature = "serialization")]
use pyo3::types::PyBytes;

#[cfg(feature = "serialization")]
use std::path::Path;

/// A document retaining its tokens, annotations, and vocabulary through Python references.
///
/// The handle is bound to its interpreter attachment, independently of the input string or pipeline.
/// Tokens and spans retain their backing document even after this handle is dropped.
pub struct Doc<'py> {
    inner: Bound<'py, PyAny>,
}

impl<'py> Doc<'py> {
    pub(crate) fn from_python(inner: Bound<'py, PyAny>) -> Self {
        Self { inner }
    }

    /// Returns the underlying Python document for operations outside the typed interface.
    pub fn as_python(&self) -> &Bound<'py, PyAny> {
        &self.inner
    }

    /// Returns the original document text, including whitespace.
    pub fn text(&self) -> Result<String, Error> {
        self.inner.getattr("text")?.extract()
    }

    /// Returns the number of tokens, including whitespace tokens.
    pub fn len(&self) -> Result<usize, Error> {
        self.inner.len()
    }

    /// Returns whether the document contains no tokens.
    pub fn is_empty(&self) -> Result<bool, Error> {
        self.len().map(|n| n == 0)
    }

    /// Returns the token at a zero-based index, or `None` when the index is out of range.
    pub fn token(&self, index: usize) -> Result<Option<Token<'py>>, Error> {
        if index >= self.len()? {
            return Ok(None);
        }
        self.inner.get_item(index).map(Token::from_python).map(Some)
    }

    /// Iterates lazily over all tokens in document order, including whitespace tokens.
    ///
    /// The iterator retains the document and terminates after its first error or exhaustion.
    pub fn tokens(&self) -> Result<Tokens<'py>, Error> {
        Tokens::from_python(&self.inner)
    }

    /// Iterates lazily over sentence spans in document order.
    ///
    /// The pipeline must supply sentence boundaries.
    /// spaCy can report missing annotations when the iterator is advanced rather than when it is created.
    /// Errors are yielded once and terminate the iterator.
    /// Whitespace is retained in each span according to spaCy's output.
    pub fn sentences(&self) -> Result<Spans<'py>, Error> {
        Spans::from_python(&self.inner.getattr("sents")?)
    }

    /// Returns whether spaCy reports an annotation, without requiring it on every token.
    ///
    /// Attribute names such as `SENT_START` and `POS` follow `Doc.has_annotation()`.
    pub fn has_annotation(&self, attr: &str) -> Result<bool, Error> {
        self.inner
            .call_method1("has_annotation", (attr,))?
            .extract()
    }

    /// Iterates over named-entity spans in document order.
    ///
    /// This reads the existing annotations; it does not run an entity recognizer.
    #[cfg(feature = "entities")]
    pub fn entities(&self) -> Result<Spans<'py>, Error> {
        Spans::from_python(&self.inner.getattr("ents")?)
    }

    /// Serializes this document into Rust-owned bytes using spaCy's `Doc.to_bytes()`.
    #[cfg(feature = "serialization")]
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let bytes = self
            .inner
            .call_method0("to_bytes")?
            .cast_into::<PyBytes>()?;
        Ok(bytes.as_bytes().to_vec())
    }

    /// Restores serialized annotations into an empty document using `Doc.from_bytes()`.
    ///
    /// Create the empty document with the vocabulary required by the serialized data.
    /// Python validates the input and the target document's state.
    /// Existing views follow spaCy's invalidation rules when the document is mutated.
    #[cfg(feature = "serialization")]
    pub fn read_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.inner
            .call_method1("from_bytes", (PyBytes::new(self.inner.py(), bytes),))?;
        Ok(())
    }

    /// Saves this document using spaCy's `Doc.to_disk()`.
    #[cfg(feature = "serialization")]
    pub fn to_disk(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        self.inner.call_method1("to_disk", (path.as_ref(),))?;
        Ok(())
    }

    /// Restores saved state into an empty document using spaCy's `Doc.from_disk()`.
    ///
    /// As with [`Self::read_bytes()`], the target must have the appropriate vocabulary and its
    /// existing views follow spaCy's mutation rules.
    #[cfg(feature = "serialization")]
    pub fn read_disk(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        self.inner.call_method1("from_disk", (path.as_ref(),))?;
        Ok(())
    }
}
