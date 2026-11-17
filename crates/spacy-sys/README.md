# spacy-sys

[![crates.io](https://img.shields.io/crates/v/spacy-sys.svg)](https://crates.io/crates/spacy-sys)
[![docs.rs](https://docs.rs/spacy-sys/badge.svg)](https://docs.rs/spacy-sys)
[![License](https://img.shields.io/crates/l/spacy-sys.svg)](https://crates.io/crates/spacy-sys)

**A general Rust wrapper for [spaCy](https://spacy.io/), using PyO3 to embed CPython.**

The wrapper provides pipelines, documents, sentences, tokens, and common annotations.
For operations outside the typed interface, `as_python()` exposes the underlying object through the re-exported `python` (PyO3) crate.
For the LiTA tokenization traits, use [lita-tokenizers-spacy](../lita-tokenizers-spacy).

---


## Supported spaCy versions

`spacy-sys` 0.1 supports spaCy **3.8.18**.
Use pipeline models and Python dependencies compatible with this release.


## Build and runtime requirements

Install a shared-library build of CPython before building this crate or a binary that depends on it.
Distribution-provided Python may also require its development/linker package, such as `python3-dev` on Debian/Ubuntu.
PyO3 discovers Python from the environment; select an interpreter explicitly with `PYO3_PYTHON`:

```sh
PYO3_PYTHON=/usr/bin/python3 cargo build -p spacy-sys
```

`PYO3_PYTHON` selects the build configuration; changing it at runtime does not change the linked Python.
Cross-compilation requires the target Python configuration described in [PyO3's build guide](https://pyo3.rs/v0.30.1/building-and-distribution).

At runtime, the application needs the corresponding Python shared library, standard library, spaCy, and any pipeline models it loads.
Python's ABI, architecture, and platform must match the installed extension modules.
The Rust build does not require spaCy or models to be installed, and does not install them.

When distributing a binary, provide a compatible Python installation or include the runtime and its startup configuration in the distribution.
The OS must be able to find libpython before the program starts; Linux deployments may need `LD_LIBRARY_PATH` in their launcher environment.
A virtual environment alone does not contain the base Python standard library.
If the launcher sets `PYTHONHOME`, it must refer to the base Python installation.

## Installation

```toml
[dependencies]
spacy-sys = "0.1.0"
```

## Usage

With spaCy already installed on the embedded interpreter's search path:

```rust,no_run
use spacy_sys::{Error, Language, Python};

let mut nlp = Language::blank("en")?;
nlp.add_pipe("sentencizer")?;
Python::attach(|py| -> Result<(), Error> {
    let doc = nlp.process(py, "Hello world. Goodbye!")?;
    assert_eq!(doc.len()?, 5);
    let sentences = doc
        .sentences()?
        .map(|sentence| sentence?.text())
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(sentences, ["Hello world.", "Goodbye!"]);
    Ok(())
})?;
# Ok::<(), Error>(())
```

`Language::blank()` creates a tokenizer without trained annotations or sentence boundaries.
Add a sentencizer or load a pipeline supplying boundaries before calling `Doc::sentences()`.
`Language::load("en_core_web_sm")` loads an installed trained pipeline; `Language::load_path(path)` loads a pipeline directory.
Models and their dependencies must already be installed.

`Language::make_doc()` runs only the tokenizer, while `Language::process()` also runs the active pipeline components.
`Language::pipe()` processes a batch lazily in input order.
Python exceptions are returned as `Error` (`pyo3::PyErr`); fallible iterators terminate after an error or exhaustion.

`Token::attribute()` reads the attribute specified by `TokenAttribute`: `Pos` for `pos_`, `Tag` for `tag_`, `Lemma` for `lemma_`, and `Morph` for `str(morph)`.
The `parsing` feature adds `Dep` (`dep_`); `entities` adds `EntIob` (`ent_iob_`) and `EntType` (`ent_type_`).
Available annotations depend on the loaded pipeline.

## Package and model locations

Install spaCy and models in a location searchable by the embedded Python.
Selecting a virtual environment with `PYO3_PYTHON` does not by itself guarantee that its site-packages will be searched.
Use `Config` to add package directories explicitly:

```rust,no_run
use spacy_sys::{Config, Error};
use std::path::Path;

let paths = [Path::new("/opt/my-app/venv/lib/python3.12/site-packages")];
let nlp = Config::new(&paths).load("en_core_web_sm")?;
# Ok::<(), Error>(())
```

Find the package directories with `sysconfig.get_path('purelib')` and `sysconfig.get_path('platlib')` in the Python used to prepare the environment.
For model packages, include the installation directory containing the package and its distribution metadata.
Loading a model directory still requires its Python dependencies and registered components to be importable.

Configured paths are added with `site.addsitedir()`, which also processes `.pth` files, including their executable lines.
Search paths and imported modules are process-wide; added paths persist after the configuration or pipeline is dropped.
Configure package locations before the first spaCy import.
Adding paths does not isolate environments or replace an already imported package.

## Ownership and costs

`Language` owns a pipeline reference.
`Doc`, `Span`, and `Token` remain valid within their `Python::attach()` scope and retain their backing data independently of the input string and pipeline.
To retain an underlying Python object beyond that scope, clone and unbind its handle, for example `nlp.as_python(py).clone().unbind()`.

String getters return Rust-owned strings.
`Token::text_ref()` and `Span::text_ref()` return Python string handles for borrowed inspection.

## Feature flags

All optional features are enabled by default:

- `entities`: Document entities, token entity annotations, and span labels.
- `parsing`: Token dependency labels, heads, and children.
- `serialization`: Pipeline/document saving and document byte serialization.

Disable them with `default-features = false`.
Tokenization, sentence iteration, POS, fine POS, lemmas, morphology, and basic pipeline operations remain available.
Features expose Rust APIs; the loaded spaCy pipeline determines which annotations are populated.

For development and testing, see [CONTRIBUTING.md](CONTRIBUTING.md).
