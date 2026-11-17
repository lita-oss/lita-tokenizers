use crate::Error;
#[cfg(feature = "parsing")]
use crate::Tokens;

use pyo3::prelude::*;
use pyo3::types::PyString;

/// A string-valued linguistic attribute provided by a spaCy token.
///
/// These are spaCy annotation kinds, not positions in an application's output.
/// In particular, [`Self::Tag`] selects spaCy's fine part-of-speech tag;
/// the other variants select distinct annotations.
/// A pipeline determines which values are available.
/// Selecting an attribute does not run or enable a pipeline component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAttribute {
    /// Coarse part-of-speech label (`Token.pos_`).
    Pos,
    /// Fine part-of-speech tag (`Token.tag_`).
    Tag,
    /// Lemma (`Token.lemma_`).
    Lemma,
    /// Morphological features (`str(Token.morph)`).
    Morph,
    /// Dependency label (`Token.dep_`).
    #[cfg(feature = "parsing")]
    Dep,
    /// Named-entity IOB marker (`Token.ent_iob_`).
    #[cfg(feature = "entities")]
    EntIob,
    /// Named-entity label (`Token.ent_type_`).
    #[cfg(feature = "entities")]
    EntType,
}

/// A spaCy token retaining its backing document through a Python reference.
///
/// This handle is valid for its interpreter attachment.
/// It does not borrow the input text, language pipeline, document handle, or iterator that produced it.
/// Attribute getters call Python directly; string getters extract owned UTF-8 strings at that boundary.
pub struct Token<'py> {
    inner: Bound<'py, PyAny>,
}

impl<'py> Token<'py> {
    pub(crate) fn from_python(inner: Bound<'py, PyAny>) -> Self {
        Self { inner }
    }

    /// Returns the underlying Python token for operations outside the typed interface.
    pub fn as_python(&self) -> &Bound<'py, PyAny> {
        &self.inner
    }

    /// Returns the original token text as an owned UTF-8 string.
    pub fn text(&self) -> Result<String, Error> {
        self.inner.getattr("text")?.extract()
    }

    /// Returns a Python string handle for borrowed inspection without extracting a Rust string.
    ///
    /// Python may still construct the string when its `Token.text` property is accessed.
    pub fn text_ref(&self) -> Result<Bound<'py, PyString>, Error> {
        Ok(self.inner.getattr("text")?.cast_into()?)
    }

    /// Returns the zero-based token index within its document.
    pub fn index(&self) -> Result<usize, Error> {
        self.inner.getattr("i")?.extract()
    }

    /// Returns the token's start offset in Unicode characters within its document.
    ///
    /// This is spaCy's `Token.idx`, not a UTF-8 byte offset.
    pub fn idx(&self) -> Result<usize, Error> {
        self.inner.getattr("idx")?.extract()
    }

    /// Returns one linguistic attribute, extracting an owned UTF-8 string at the Python boundary.
    ///
    /// This delegates to the corresponding getter without additional allocation or processing.
    /// Missing annotations follow that getter's spaCy representation, generally an empty string.
    pub fn attribute(&self, kind: TokenAttribute) -> Result<String, Error> {
        match kind {
            TokenAttribute::Pos => self.pos(),
            TokenAttribute::Tag => self.tag(),
            TokenAttribute::Lemma => self.lemma(),
            TokenAttribute::Morph => self.morph(),
            #[cfg(feature = "parsing")]
            TokenAttribute::Dep => self.dep(),
            #[cfg(feature = "entities")]
            TokenAttribute::EntIob => self.ent_iob(),
            #[cfg(feature = "entities")]
            TokenAttribute::EntType => self.ent_type(),
        }
    }

    /// Returns the coarse part-of-speech label, or an empty string when unannotated.
    pub fn pos(&self) -> Result<String, Error> {
        self.inner.getattr("pos_")?.extract()
    }

    /// Returns the fine part-of-speech tag, or an empty string when unannotated.
    pub fn tag(&self) -> Result<String, Error> {
        self.inner.getattr("tag_")?.extract()
    }

    /// Returns the lemma, or an empty string when unannotated.
    pub fn lemma(&self) -> Result<String, Error> {
        self.inner.getattr("lemma_")?.extract()
    }

    /// Returns spaCy's string representation of the morphological features.
    ///
    /// Unannotated tokens yield an empty string.
    /// Feature formatting and ordering follow spaCy's `MorphAnalysis.__str__()` implementation.
    pub fn morph(&self) -> Result<String, Error> {
        self.inner.getattr("morph")?.str()?.extract()
    }

    /// Returns whether the token consists of whitespace.
    pub fn is_space(&self) -> Result<bool, Error> {
        self.inner.getattr("is_space")?.extract()
    }

    /// Returns whether the token consists of punctuation.
    pub fn is_punct(&self) -> Result<bool, Error> {
        self.inner.getattr("is_punct")?.extract()
    }

    /// Returns whether the token consists of alphabetic characters.
    pub fn is_alpha(&self) -> Result<bool, Error> {
        self.inner.getattr("is_alpha")?.extract()
    }

    /// Returns whether spaCy marks the token as a stop word for its language.
    pub fn is_stop(&self) -> Result<bool, Error> {
        self.inner.getattr("is_stop")?.extract()
    }

    /// Returns the sentence-start annotation, or `None` when the boundary is unknown.
    pub fn is_sent_start(&self) -> Result<Option<bool>, Error> {
        self.inner.getattr("is_sent_start")?.extract()
    }

    /// Returns the sentence-end annotation, or `None` when the boundary is unknown.
    pub fn is_sent_end(&self) -> Result<Option<bool>, Error> {
        self.inner.getattr("is_sent_end")?.extract()
    }

    /// Returns the dependency label, or an empty string when unannotated.
    #[cfg(feature = "parsing")]
    pub fn dep(&self) -> Result<String, Error> {
        self.inner.getattr("dep_")?.extract()
    }

    /// Returns the syntactic head, retaining the backing document through Python.
    ///
    /// A root token is its own head.
    /// Without dependency annotations, spaCy's existing head value is returned; this method does not run a parser.
    #[cfg(feature = "parsing")]
    pub fn head(&self) -> Result<Self, Error> {
        self.inner.getattr("head").map(Self::from_python)
    }

    /// Iterates over immediate dependency children in spaCy's order.
    ///
    /// The iterator retains its Python generator and terminates after an error or exhaustion.
    #[cfg(feature = "parsing")]
    pub fn children(&self) -> Result<Tokens<'py>, Error> {
        Tokens::from_python(&self.inner.getattr("children")?)
    }

    /// Returns spaCy's entity IOB marker, or an empty string when unannotated.
    #[cfg(feature = "entities")]
    pub fn ent_iob(&self) -> Result<String, Error> {
        self.inner.getattr("ent_iob_")?.extract()
    }

    /// Returns the named-entity label, or an empty string when none is assigned.
    #[cfg(feature = "entities")]
    pub fn ent_type(&self) -> Result<String, Error> {
        self.inner.getattr("ent_type_")?.extract()
    }
}
