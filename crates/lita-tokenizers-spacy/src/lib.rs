//! An adapter from [`spacy-sys`](spacy_sys) to [`lita-tokenizers-core`](tokenizers_core).
//!
//! Construct and configure a [`sys::Language`], then specify which [`TokenAttribute`] values become core tags.
//! These are spaCy token attributes, such as `pos_`, `tag_`, and `lemma_`,
//! rather than indices into an existing tag sequence.
//! [`Spacy::from()`] selects POS, fine POS, lemma, and morphology;
//! [`Spacy::new()`] accepts an explicit attribute sequence.
//!
//! [`TokenStreamBuilder::tokenize`] processes the input and extracts only the selected attributes
//! into Rust-owned strings at the FFI boundary.
//! Python errors are returned before the stream: the core traits provide infallible token access.
//! Subsequent iteration borrows these strings without allocating or accessing Python.
//! Streams do not borrow the input, builder, or pipeline.
//!
//! Python, spaCy, and any trained pipeline must already be available.
//! The language must provide sentence boundaries through a parser, sentence recognizer, or sentencizer.
//! Whitespace tokens are omitted, punctuation is retained,
//! and the final retained token of each sentence is marked [`TokenPosition::Eos`].
//! Missing annotations remain empty strings.
//!
//! # Examples
//!
//! ```no_run
//! use lita_tokenizers_spacy::{Spacy, TokenAttribute, sys};
//! use tokenizers_core::{Token, TokenStream, TokenStreamBuilder, Tokenizer};
//!
//! let model = sys::Language::load("en_core_web_sm")?;
//! let mut tokenizer = Spacy::new(model, [TokenAttribute::Pos, TokenAttribute::Lemma]);
//! let mut builder = tokenizer.builder()?;
//! let stream = builder.tokenize("We are programmers.")?;
//! let tags: Vec<_> = stream.tokens().next().unwrap().1.tags()
//!     .map(|t| t.as_str())
//!     .collect();
//! assert_eq!(tags, ["PRON", "we"]);
//! # Ok::<(), lita_tokenizers_spacy::Error>(())
//! ```

use tokenizers_core::{Tag, Token, TokenPosition, TokenStream, TokenStreamBuilder, Tokenizer};

pub use spacy_sys as sys;
pub use sys::{Error, TokenAttribute};

use std::iter::{Copied, FusedIterator};
use std::slice::Iter as SliceIter;

/// Specifies which spaCy token attributes become core tags.
///
/// Each call produces a fresh, finite iterator specifying the attributes to extract for one
/// retained token, in the order yielded.
/// Repeated entries request repeated values, and an empty iterator requests no tags.
/// Missing annotations produce empty strings; the specification does not enable pipeline components.
///
/// An iterator can compute, filter, or reorder attribute kinds lazily and can borrow its source.
/// It need not be cloneable, have an exact length, or be backed by a collection.
/// A builder requests a fresh iterator for each retained token and preserves that enumeration's
/// actual order and length, even if later enumerations differ.
///
/// # Examples
///
/// ```
/// use lita_tokenizers_spacy::{TokenAttribute, TokenAttributes};
/// use std::iter::{Chain, Once, once};
///
/// struct PosAndLemma;
///
/// impl TokenAttributes for PosAndLemma {
///     type Iter<'a> = Chain<Once<TokenAttribute>, Once<TokenAttribute>>;
///
///     fn attributes(&self) -> Self::Iter<'_> {
///         once(TokenAttribute::Pos).chain(once(TokenAttribute::Lemma))
///     }
/// }
///
/// assert_eq!(
///     PosAndLemma.attributes().collect::<Vec<_>>(),
///     [TokenAttribute::Pos, TokenAttribute::Lemma],
/// );
/// ```
pub trait TokenAttributes {
    /// An iterator of spaCy attribute kinds, possibly borrowing this specification.
    type Iter<'a>: Iterator<Item = TokenAttribute>
    where
        Self: 'a;

    /// Starts a finite enumeration of the spaCy attribute kinds to extract for one token.
    fn attributes(&self) -> Self::Iter<'_>;
}

impl TokenAttributes for [TokenAttribute] {
    type Iter<'a> = Copied<SliceIter<'a, TokenAttribute>>;

    fn attributes(&self) -> Self::Iter<'_> {
        self.iter().copied()
    }
}

impl<const N: usize> TokenAttributes for [TokenAttribute; N] {
    type Iter<'a> = Copied<SliceIter<'a, TokenAttribute>>;

    fn attributes(&self) -> Self::Iter<'_> {
        self.iter().copied()
    }
}

impl TokenAttributes for Vec<TokenAttribute> {
    type Iter<'a> = Copied<SliceIter<'a, TokenAttribute>>;

    fn attributes(&self) -> Self::Iter<'_> {
        self.iter().copied()
    }
}

impl<T: TokenAttributes + ?Sized> TokenAttributes for &T {
    type Iter<'a>
        = T::Iter<'a>
    where
        Self: 'a;

    fn attributes(&self) -> Self::Iter<'_> {
        <T as TokenAttributes>::attributes(*self)
    }
}

impl<T: TokenAttributes + ?Sized> TokenAttributes for &mut T {
    type Iter<'a>
        = T::Iter<'a>
    where
        Self: 'a;

    fn attributes(&self) -> Self::Iter<'_> {
        <T as TokenAttributes>::attributes(&**self)
    }
}

/// A spaCy pipeline whose selected token attributes become core tags.
///
/// The attribute source is stored as provided.
/// Processing accepts any source implementing [`TokenAttributes`],
/// including arrays, vectors, borrowed slices, and application-defined sources.
pub struct Spacy<A = &'static [TokenAttribute]> {
    model: sys::Language,
    attributes: A,
}

impl<A> Spacy<A> {
    /// Takes ownership of a pipeline and the spaCy attributes to extract for each token.
    ///
    /// Core tags follow the supplied sequence.
    /// An empty sequence produces surfaces without tags.
    /// Missing annotations produce empty strings; selecting an attribute does not run or enable
    /// the pipeline component that supplies it.
    ///
    /// The attribute source is moved into the tokenizer without copying or converting it.
    /// Borrowed sources must remain valid while the tokenizer uses them.
    pub fn new(model: sys::Language, attributes: A) -> Self {
        Self { model, attributes }
    }

    /// Borrows the pipeline for inspection or configuration before processing more input.
    pub fn model(&mut self) -> &mut sys::Language {
        &mut self.model
    }
}

impl<A: TokenAttributes> Spacy<A> {
    /// Returns a fresh iterator from [`TokenAttributes::attributes()`] in extraction order.
    pub fn attributes(&self) -> A::Iter<'_> {
        self.attributes.attributes()
    }
}

impl<A: TokenAttributes> Tokenizer for Spacy<A> {
    type StreamBuilder<'t>
        = SpacyStreamBuilder<'t, A>
    where
        Self: 't;
    type Error = Error;

    fn builder<'t>(&'t mut self) -> Result<Self::StreamBuilder<'t>, Self::Error> {
        Ok(SpacyStreamBuilder {
            model: &self.model,
            attributes: &self.attributes,
        })
    }
}

/// Borrows the pipeline and selected spaCy attributes while processing input.
pub struct SpacyStreamBuilder<'t, A = &'static [TokenAttribute]> {
    model: &'t sys::Language,
    attributes: &'t A,
}

impl<A: TokenAttributes> TokenStreamBuilder for SpacyStreamBuilder<'_, A> {
    type Output<'t, 's>
        = SpacyStream
    where
        Self: 't;
    type Error = Error;

    fn tokenize<'t, 's>(&'t mut self, input: &'s str) -> Result<Self::Output<'t, 's>, Self::Error> {
        sys::Python::attach(|py| {
            let doc = self.model.process(py, input)?;
            let mut tokens = Vec::new();
            let mut tags = Vec::new();
            for sentence in doc.sentences()? {
                let start = tokens.len();
                for token in sentence?.tokens()? {
                    let token = token?;
                    if token.is_space()? {
                        continue;
                    }

                    let surface = token.text()?;
                    let tag_start = tags.len();
                    for attribute in self.attributes.attributes() {
                        tags.push(token.attribute(attribute)?);
                    }
                    // The loop only appends, so tags.len() >= tag_start and subtraction cannot underflow.
                    let tag_count = tags.len() - tag_start;
                    tokens.push(TokenData {
                        position: TokenPosition::Normal,
                        surface,
                        tag_count,
                    });
                }

                // A sentence can end in whitespace. Its boundary belongs to its final retained
                // token; an entirely omitted sentence must not mark the preceding sentence.
                if let Some(last) = tokens.get_mut(start..).and_then(|s| s.last_mut()) {
                    last.position = TokenPosition::Eos;
                }
            }

            Ok(SpacyStream { tokens, tags })
        })
    }
}

struct TokenData {
    position: TokenPosition,
    surface: String,
    tag_count: usize,
}

/// Rust-owned surfaces and selected attributes, independent of the input and pipeline.
///
/// The input, builder, and tokenizer may be dropped or reused.
/// This stream and its iterators do not access Python.
/// Attribute values for all tokens share a single buffer.
pub struct SpacyStream {
    tokens: Vec<TokenData>,
    // The per-token counts partition this immutable buffer in token order.
    tags: Vec<String>,
}

impl TokenStream for SpacyStream {
    type IntoIter<'o> = SpacyTokenIter<'o>;
    type Token<'o> = SpacyToken<'o>;

    fn tokens<'o>(&'o self) -> Self::IntoIter<'o> {
        SpacyTokenIter {
            tokens: self.tokens.iter(),
            tags: &self.tags,
        }
    }
}

/// Iterates over stored tokens without accessing Python or allocating.
pub struct SpacyTokenIter<'o> {
    tokens: SliceIter<'o, TokenData>,
    tags: &'o [String],
}

impl<'o> Iterator for SpacyTokenIter<'o> {
    type Item = (TokenPosition, SpacyToken<'o>);

    fn next(&mut self) -> Option<Self::Item> {
        let data = self.tokens.next()?;
        // Each token records exactly the number of strings appended for it. The private stream
        // buffers are never mutated, so these counts partition the complete tag buffer.
        // Each preceding next() consumed one token and its count of strings; therefore this
        // token's recorded count is at most the remaining slice length, including a zero count.
        let (tags, rest) = self.tags.split_at(data.tag_count);
        self.tags = rest;
        Some((
            data.position,
            SpacyToken {
                surface: &data.surface,
                tags,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tokens.size_hint()
    }
}

impl ExactSizeIterator for SpacyTokenIter<'_> {}
impl FusedIterator for SpacyTokenIter<'_> {}

/// A borrowed view of a stored surface and its selected spaCy attribute values.
pub struct SpacyToken<'o> {
    surface: &'o str,
    tags: &'o [String],
}

impl<'o> Token<'o> for SpacyToken<'o> {
    type Tags = SpacyTags<'o>;

    fn surface(&self) -> Tag<'o> {
        Tag::Unescaped(self.surface)
    }

    fn tags(&self) -> Self::Tags {
        SpacyTags {
            tags: self.tags.iter(),
        }
    }
}

/// Iterates over the selected attribute values, including empty annotations.
pub struct SpacyTags<'o> {
    tags: SliceIter<'o, String>,
}

impl<'o> Iterator for SpacyTags<'o> {
    type Item = Tag<'o>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tags.next().map(|tag| Tag::Unescaped(tag))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tags.size_hint()
    }
}

impl ExactSizeIterator for SpacyTags<'_> {}
impl FusedIterator for SpacyTags<'_> {}
