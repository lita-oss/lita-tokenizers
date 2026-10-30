# lita-tokenizers-utils

[![crates.io](https://img.shields.io/crates/v/lita-tokenizers-utils.svg)](https://crates.io/crates/lita-tokenizers-utils)
[![docs.rs](https://docs.rs/lita-tokenizers-utils/badge.svg)](https://docs.rs/lita-tokenizers-utils)
[![License](https://img.shields.io/crates/l/lita-tokenizers-utils.svg)](https://crates.io/crates/lita-tokenizers-utils)

Exposes the high-level APIs of `lita-tokenizers-core`.

Read the module-level documentation for more details.

## Feature flags

This crate has several feature flags:

- `str` (*enabled by default*): utilities for pure string manipulation.
- `decode` (*disabled by default*): When disabled, we accept only UTF-8 inputs via [`String::from_utf8_lossy()`)(https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8_lossy). When enabled, you can pass an input text in other encodings.
- `write` (*enabled by default*): utilities to write a tokenized output as a single string.
- `sync-io` (*disabled by default*): wraps an input/output by `std::io::{Read, Write}`. It implies `write`.
- `async-io` (*disabled by default*): wraps an input/output by `futures::io::{AsyncRead, AsyncWrite}`. It implies `write`.
- `tokio-io` (*disabled by default*): wraps an input/output by `tokio::io::{AsyncRead, AsyncWrite}`. It implies `write`.
- `sync-fs` (*disabled by default*): wraps an input/output by `std::fs`. It implies `sync-io`.
- `smol-fs` (*disabled by default*): wraps an input/output by `smol::fs`. It implies `async-io`.
- `tokio-fs` (*disabled by default*): wraps an input/output by `tokio::fs`. It implies `tokio-io`.
