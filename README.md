# lita-tokenizers

Providing the detailed APIs of tokenizers in the LiTA app.

It includes both the [Rust endpoint](./crates/lita-tokenizers) as a lib crate and the [CLI](./crates/lita-tokenizers-cli).

The standalone [spacy-sys](./crates/spacy-sys) crate wraps spaCy for general Rust applications.
[lita-tokenizers-spacy](./crates/lita-tokenizers-spacy) adapts it to the core tokenizer traits; this backend is not yet exposed by the facade or CLI.
