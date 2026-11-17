//! Typed wrappers for spaCy using an embedded CPython interpreter.
//!
//! [`Language`] owns a reusable pipeline.
//! Processing takes a [`Python`] attachment and returns [`Doc`], [`Span`], and [`Token`] handles
//! which retain their underlying Python objects.
//! The handles are valid for that attachment, independently of the pipeline or originating handle.
//! Iterators visit Python objects lazily and return Python exceptions as [`Error`].
//!
//! The wrappers do not copy documents or annotations eagerly.
//! String getters extract owned UTF-8 strings at the Python boundary;
//! `text_ref()` returns a Python string handle for borrowed inspection instead.
//! Iterator adapters and handle access introduce no Rust heap allocations or dynamic dispatch.
//! Python calls and reference counting retain their normal costs.
//!
//! The caller supplies shared CPython at build and runtime, spaCy, and any models.
//! Nothing is downloaded or installed by this crate.
//! [`Config`] can add existing site-packages directories;
//! these paths and Python's imported-module cache are shared by the process.
//!
//! The default `entities`, `parsing`, and `serialization` features expose additional annotation
//! and persistence APIs.
//! Tokenization, sentence spans, POS, tags, lemmas, and morphology remain available with default features disabled.
//! A loaded pipeline determines which annotations are populated;
//! enabling a Rust feature does not add Python pipeline components.
//!
//! # Examples
//!
//! ```no_run
//! use spacy_sys::{Error, Language, Python};
//!
//! let mut nlp = Language::blank("en")?;
//! nlp.add_pipe("sentencizer")?;
//!
//! Python::attach(|py| -> Result<(), Error> {
//!     let doc = nlp.process(py, "Hello, world!")?;
//!
//!     let text = doc.tokens()?
//!         .map(|token| token?.text())
//!         .collect::<Result<Vec<_>, _>>()?;
//!
//!     assert_eq!(text, ["Hello", ",", "world", "!"]);
//!
//!     Ok(())
//! })?;
//! # Ok::<(), Error>(())
//! ```

mod config;
pub use config::Config;

mod language;
pub use language::Language;

mod doc;
pub use doc::Doc;

mod span;
pub use span::Span;

mod token;
pub use token::{Token, TokenAttribute};

mod iter;
pub use iter::{Docs, Spans, Tokens};

pub use pyo3 as python;
pub use pyo3::{PyErr as Error, Python};
