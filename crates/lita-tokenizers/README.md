# `lita-tokenizers`

[![crates.io](https://img.shields.io/crates/v/lita-tokenizers.svg)](https://crates.io/crates/lita-tokenizers)
[![docs.rs](https://docs.rs/lita-tokenizers/badge.svg)](https://docs.rs/lita-tokenizers)
[![License](https://img.shields.io/crates/l/lita-tokenizers.svg)](https://crates.io/crates/lita-tokenizers)

High-level APIs to utilize tokenizers that are working on the LiTA app.

This crate gives you a set of tokenizer-agnostic APIs in [`common`],
along with tokenier-specific entrypoints in [`kytea`], [`mecab`], and [`whitespace`].
These tokenizers can be opt-out via feature flags `kytea`/`mecab` because they depends on the
C++ bindings `kytea-sys`/`mecab-sys` respectively, hence requiring the complex build-time
environment.

## Async runtime

We heavily rely on [`tokio`] as an async runtime, especially for IO and filesystem operations.
If you would like other async runtimes like `smol` or execute your codes in a blocking way (`std`),
please use the runtime-independent, internal crate [`lita-tokenizers-utils`](utils) directly.

## Feature flags

All of the features are *enabled by default*.

- `decode`: When disabled, input text are interpreted as UTF-8; when enabled, it will be aware of other encodings. See the documentation of [`utils`].
- `kytea`: When enabled, we offer the rich APIs for [KyTea](kytea_core) tokenier.
- `mecab`: When enabled, we offer the rich APIs for [MeCab](mecab_core) tokenier.

If you enable the C/C++-related features, `kytea` or `mecab`, you may need the additional
build-time requirements. Check the documentation of the `kytea-sys` and `mecab-sys` crates.
