#![cfg_attr(docsrs, feature(doc_cfg))]

//! High-level APIs to utilize tokenizers that are working on the LiTA app.
//!
//! This crate gives you a set of tokenizer-agnostic APIs in [`common`],
//! along with tokenier-specific entrypoints in [`kytea`], [`mecab`], [`spacy`], and [`whitespace`].
//! These tokenizers can be opt-out via feature flags `kytea`/`mecab`/`spacy` because they depends on the
//! C++ bindings `kytea-sys`/`mecab-sys` and the Python bindings `spacy-sys` respectively,
//! hence requiring the complex build-time environment.
//!
//! # Async runtime
//!
//! We heavily rely on [`tokio`] as an async runtime, especially for IO and filesystem operations.
//! If you would like other async runtimes like `smol` or execute your codes in a blocking way (`std`),
//! please use the runtime-independent, internal crate [`lita-tokenizers-utils`](utils) directly.
//!
//! # Feature flags
//!
//! All of the features are *enabled by default*.
//!
//! - `decode`: When disabled, input text are interpreted as UTF-8; when enabled, it will be aware of other encodings. See the documentation of [`utils`].
//! - `kytea`: When enabled, we offer the rich APIs for [KyTea](kytea_core) tokenier.
//! - `mecab`: When enabled, we offer the rich APIs for [MeCab](mecab_core) tokenier.
//! - `spacy`: When enabled, we offer the rich APIs for [spaCy](spacy_core) tokenier.
//!
//! If you enable the C/C++/Python-related features, `kytea`, `mecab`, or `spacy`, you may need the additional
//! build-time requirements. Check the documentation of the `kytea-sys`, `mecab-sys`, and `spacy-sys` crates.

mod error;
pub use error::Error;

pub mod common;
#[cfg(feature = "tokio")]
pub use common::sync::{tokenize, tokenize_stream};
pub use common::{Input, Output, TokenizerError};

#[cfg(feature = "kytea")]
pub mod kytea;
#[cfg(feature = "mecab")]
pub mod mecab;
#[cfg(feature = "spacy")]
pub mod spacy;
pub mod whitespace;

pub use tokenizers_core as core;

pub use tokenizers_utils as utils;
pub use utils::Charset;

#[cfg(feature = "kytea")]
pub use tokenizers_kytea as kytea_core;

#[cfg(feature = "mecab")]
pub use tokenizers_mecab as mecab_core;

#[cfg(feature = "spacy")]
pub use tokenizers_spacy as spacy_core;

pub use tokenizers_whitespace as whitespace_core;
