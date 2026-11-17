# lita-tokenizers-spacy

[![crates.io](https://img.shields.io/crates/v/lita-tokenizers-spacy.svg)](https://crates.io/crates/lita-tokenizers-spacy)
[![docs.rs](https://docs.rs/lita-tokenizers-spacy/badge.svg)](https://docs.rs/lita-tokenizers-spacy)
[![License](https://img.shields.io/crates/l/lita-tokenizers-spacy.svg)](https://crates.io/crates/lita-tokenizers-spacy)

The implementation of `lita-tokenizers-core` for [spacy-sys](../spacy-sys).
This is a low-level backend; it is not yet integrated into the `lita-tokenizers` facade or `lita-tokenizers-cli`.

The crate depends only on `lita-tokenizers-core` and `spacy-sys`.
It does not install Python, spaCy, or pipeline models.
The shared Python library is required at build and runtime; spaCy and its models are required when loading and running a pipeline.
See [spacy-sys](../spacy-sys#build-and-runtime-requirements) for Python selection, linking, package search paths, and application distribution.

## Usage

```toml
[dependencies]
lita-tokenizers-spacy = "0.1.0"
tokenizers-core = { package = "lita-tokenizers-core", version = "0.1.0" }
```

Create a spaCy pipeline, then adapt it to the core traits:

```rust,no_run
use lita_tokenizers_spacy::{sys, Spacy};
use tokenizers_core::{Token, TokenPosition, TokenStream, TokenStreamBuilder, Tokenizer};

let mut model = sys::Language::blank("en")?;
model.add_pipe("sentencizer")?;
let mut tokenizer = Spacy::from(model);
let mut builder = tokenizer.builder()?;
let stream = builder.tokenize("Hello world!")?;
assert_eq!(
    stream
        .tokens()
        .map(|(position, token)| (position, token.surface().as_str()))
        .collect::<Vec<_>>(),
    [
        (TokenPosition::Normal, "Hello"),
        (TokenPosition::Normal, "world"),
        (TokenPosition::Eos, "!"),
    ],
);
# Ok::<(), sys::Error>(())
```

Use `sys::Config` to add site-packages directories and load an installed model, or use `sys::Language::load_path()` for a model directory.
The pipeline must supply sentence boundaries through a parser, sentence recognizer, or sentencizer.
Absent boundaries, model-loading failures, and processing exceptions are returned as the concrete `Error` re-exported from `spacy-sys`.

## Output and ownership

Token surfaces preserve the original text.
Whitespace tokens are omitted and punctuation is retained.
The last retained token in each sentence is marked `TokenPosition::Eos`, including when spaCy's sentence ends with whitespace.

Specify spaCy's token attributes by name, using `TokenAttribute` from `spacy-sys`:

```rust,no_run
use lita_tokenizers_spacy::{sys, Spacy, TokenAttribute};

let model = sys::Language::load("en_core_web_sm")?;
let attributes = [TokenAttribute::Pos, TokenAttribute::Lemma];
let tokenizer = Spacy::new(model, &attributes);
assert_eq!(
    tokenizer.attributes().collect::<Vec<_>>(),
    [TokenAttribute::Pos, TokenAttribute::Lemma],
);
# Ok::<(), sys::Error>(())
```

`Pos` extracts `Token.pos_` (coarse POS), `Tag` extracts `Token.tag_` (fine POS), `Lemma` extracts `Token.lemma_`, and `Morph` extracts `str(Token.morph)`.
These are distinct [spaCy token attributes](https://spacy.io/api/token#attributes).
Each becomes a core tag in the supplied sequence; the adapter extracts only the requested attributes.
An empty sequence produces surfaces without tags.
Missing annotations are empty strings.
Selecting an attribute does not enable or run a pipeline component that supplies it.

`Spacy::from(model)` is the adapter's shorthand for `[Pos, Tag, Lemma, Morph]`.
A blank English pipeline with a sentencizer supplies empty values for these four attributes.
This default is a choice of this adapter, not a spaCy tag ordering.

`Spacy::new()` accepts an extraction specification implementing `TokenAttributes`.
Arrays, vectors, borrowed slices, borrowed vectors, and custom types implementing that trait can supply the sequence.
The trait returns an iterator identifying which spaCy attribute value becomes each core tag, in the iterator's order.
The iterator must be finite; repeated attributes repeat the corresponding tag, and an empty iterator omits tags.
The iterator can compute, filter, or reorder attributes lazily, and may borrow its source.
It need not be cloneable, have an exact length, or be backed by a collection.
The builder borrows the source and requests a fresh iterator for each retained token; it does not freeze the sequence when constructed.
Order and length may differ between enumerations; each token retains the values actually extracted for it.
Configuration changes affect later enumerations; results already extracted into a stream remain unchanged.
The tokenizer stores the supplied source unchanged, without copying, converting, or boxing it.
Borrowed sources must remain valid while the tokenizer uses them.
The selection can be determined at runtime.
Attribute values for all tokens share one buffer in the stream, avoiding a separate allocation for each token's tag collection.

An application can generate its ordering lazily without storing a slice or collecting an intermediate sequence:

```rust,no_run
use lita_tokenizers_spacy::{sys, Spacy, TokenAttribute, TokenAttributes};

use std::iter::{once, Chain, Once};

struct Order {
    lemma_first: bool,
}

impl TokenAttributes for Order {
    type Iter<'a> = Chain<Once<TokenAttribute>, Once<TokenAttribute>>
    where
        Self: 'a;

    fn attributes(&self) -> Self::Iter<'_> {
        let (first, second) = if self.lemma_first {
            (TokenAttribute::Lemma, TokenAttribute::Pos)
        } else {
            (TokenAttribute::Pos, TokenAttribute::Lemma)
        };
        once(first).chain(once(second))
    }
}

let model = sys::Language::load("en_core_web_sm")?;
let tokenizer = Spacy::new(model, Order { lemma_first: true });
assert_eq!(
    tokenizer.attributes().collect::<Vec<_>>(),
    [TokenAttribute::Lemma, TokenAttribute::Pos],
);
# Ok::<(), sys::Error>(())
```

The core traits expose infallible, borrowed token access.
Consequently, `tokenize()` performs all fallible Python work under one interpreter attachment and extracts the surfaces and annotations into an owned Rust buffer.
This materialization is the FFI boundary; subsequent token/tag iteration neither accesses Python nor allocates, copies strings, or uses dynamic dispatch.

Streams own their results independently of the input, builder, and tokenizer.
They remain usable after those values are dropped or reused.
`model()` exposes the underlying pipeline for configuration before building a stream.

The backend disables `spacy-sys`'s optional features because it needs only the mandatory interface.
Applications can enable additional `spacy-sys` features through their own dependency: `parsing` adds `TokenAttribute::Dep`, and `entities` adds `TokenAttribute::EntIob` and `TokenAttribute::EntType`.

For integration tests and environment setup, see [spacy-sys development instructions](../spacy-sys/CONTRIBUTING.md).
