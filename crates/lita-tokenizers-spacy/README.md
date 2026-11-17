# lita-tokenizers-spacy

[![crates.io](https://img.shields.io/crates/v/lita-tokenizers-spacy.svg)](https://crates.io/crates/lita-tokenizers-spacy)
[![docs.rs](https://docs.rs/lita-tokenizers-spacy/badge.svg)](https://docs.rs/lita-tokenizers-spacy)
[![License](https://img.shields.io/crates/l/lita-tokenizers-spacy.svg)](https://crates.io/crates/lita-tokenizers-spacy)

The implementation of `lita-tokenizers-core` for [spacy-sys](../spacy-sys).
This is a low-level backend; it is not yet integrated into the `lita-tokenizers` facade or `lita-tokenizers-cli`.

It does not install Python, spaCy, or pipeline models.
The shared Python library is required at build and runtime; spaCy and its models are required when loading and running a pipeline.
See [spacy-sys](../spacy-sys#build-and-runtime-requirements) for Python selection, linking, package search paths, and application distribution.

## Installation

```toml
[dependencies]
lita-tokenizers-spacy = "0.1.0"
tokenizers-core = { package = "lita-tokenizers-core", version = "0.1.0" }
```

## Usage

### Write token surfaces, POS, and lemmas as TSV

Prepare spaCy and `en_core_web_sm` 3.8.0 using the [Python environment instructions](../spacy-sys#python-environment).
Keep the resulting environment variables in the shell used to build and run this program.

This program reads a UTF-8 file and writes one token per line, with the original surface, coarse POS, and lemma separated by tabs.
A blank line separates sentences.
Save it as `src/main.rs` in a binary project with the dependencies above:

```rust
use lita_tokenizers_spacy::{sys, Spacy, TokenAttribute};
use tokenizers_core::{Token, TokenPosition, TokenStream, TokenStreamBuilder, Tokenizer};

use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let input = env::args_os().nth(1).ok_or("usage: spacy-tokens INPUT")?;
    let text = fs::read_to_string(PathBuf::from(input))?;

    let package_dirs = env::var_os("SPACY_SITE_PACKAGES")
        .ok_or("set SPACY_SITE_PACKAGES to the Python package directories")?;

    let model = sys::Config::new(env::split_paths(&package_dirs)).load("en_core_web_sm")?;
    let attributes = [TokenAttribute::Pos, TokenAttribute::Lemma];
    let mut tokenizer = Spacy::new(model, attributes);

    let mut builder = tokenizer.builder()?;
    let stream = builder.tokenize(&text)?;

    let mut out = io::BufWriter::new(io::stdout().lock());

    for (position, token) in stream.tokens() {
        write!(out, "{}", token.surface().as_str())?;

        for tag in token.tags() {
            write!(out, "\t{}", tag.as_str())?;
        }

        writeln!(out)?;

        if position == TokenPosition::Eos {
            writeln!(out)?;
        }
    }

    out.flush()?;

    Ok(())
}
```

Create an input file and redirect the output to a TSV file:

```sh
printf '%s\n' 'The cats are sleeping. A dog is running.' > sentences.txt
cargo run --quiet -- sentences.txt > tokens.tsv
cat tokens.tsv
```

Output:

```text
The	DET	the
cats	NOUN	cat
are	AUX	be
sleeping	VERB	sleep
.	PUNCT	.

A	DET	a
dog	NOUN	dog
is	AUX	be
running	VERB	run
.	PUNCT	.

```

The output uses `en_core_web_sm` 3.8.0; annotations depend on the loaded pipeline.
To include fine POS between coarse POS and lemma, change the attribute sequence to `[TokenAttribute::Pos, TokenAttribute::Tag, TokenAttribute::Lemma]`.
The output loop writes all selected values in that sequence, so no other formatting change is needed.

## Selecting attributes

`Pos` extracts `Token.pos_` (coarse POS), `Tag` extracts `Token.tag_` (fine POS), `Lemma` extracts `Token.lemma_`, and `Morph` extracts `str(Token.morph)`.
These are distinct [spaCy token attributes](https://spacy.io/api/token#attributes).
An empty sequence produces surfaces without tags, and missing annotations are empty strings.
Selecting an attribute does not enable or run the pipeline component that supplies it.
A blank pipeline with a sentencizer provides tokenization and sentence boundaries, but has no trained POS or lemma annotations.

`Spacy::new()` accepts any extraction specification implementing `TokenAttributes`, including arrays, vectors, and borrowed slices.
For computed ordering, implement the trait's associated iterator; see the [API reference](https://docs.rs/lita-tokenizers-spacy/0.1.0/lita_tokenizers_spacy/trait.TokenAttributes.html).
The iterator must be finite and may compute, filter, or reorder attributes lazily without being cloneable or having an exact length.
The builder requests a fresh iterator for each retained token.
Each token retains the values actually extracted for it, even when order or length differs between enumerations.

To use `TokenAttribute::Dep`, enable `spacy-sys`'s `parsing` feature through a direct dependency.
To use `TokenAttribute::EntIob` or `TokenAttribute::EntType`, enable its `entities` feature.

## Output and ownership

Token surfaces preserve the original text.
Whitespace tokens are omitted and punctuation is retained.
The last retained token in each sentence is marked `TokenPosition::Eos`, including when spaCy's sentence ends with whitespace.
The pipeline must supply sentence boundaries through a parser, sentence recognizer, or sentencizer.

`tokenize()` returns Python processing errors before returning a stream.
Streams own their results independently of the input, builder, tokenizer, and attribute source.
They remain usable after those values are dropped or reused, and iterating over the results does not access Python.
Borrowed attribute sources must remain valid while the tokenizer uses them.
`model()` exposes the underlying pipeline for configuration before building a stream.

For development and testing, see [spacy-sys development instructions](../spacy-sys/CONTRIBUTING.md).
