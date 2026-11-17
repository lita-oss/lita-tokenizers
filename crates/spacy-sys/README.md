# spacy-sys

[![crates.io](https://img.shields.io/crates/v/spacy-sys.svg)](https://crates.io/crates/spacy-sys)
[![docs.rs](https://docs.rs/spacy-sys/badge.svg)](https://docs.rs/spacy-sys)
[![License](https://img.shields.io/crates/l/spacy-sys.svg)](https://crates.io/crates/spacy-sys)

**A general Rust wrapper for [spaCy](https://spacy.io/), using PyO3 to embed CPython.**

The wrapper provides pipelines, documents, sentences, tokens, and common annotations.

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

### Python environment

The following examples use a Linux shell and a shared-library CPython installation with `venv` and pip available.
Run these commands in your Rust application's directory to install spaCy and the English pipeline into a local environment:

```sh
unset PYTHONHOME PYTHONPATH
python3 -m venv .venv

.venv/bin/python -m pip install \
    'spacy==3.8.18' \
    'https://github.com/explosion/spacy-models/releases/download/en_core_web_sm-3.8.0/en_core_web_sm-3.8.0-py3-none-any.whl'

export PYO3_PYTHON="$PWD/.venv/bin/python"

export SPACY_SITE_PACKAGES="$(
    "$PYO3_PYTHON" -c 'import os, site; print(os.pathsep.join(site.getsitepackages()))'
)"

python_home="$("$PYO3_PYTHON" -c 'import sys; print(sys.base_prefix)')"
python_lib="$("$PYO3_PYTHON" -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')"

export PYTHONHOME="$python_home"
export LD_LIBRARY_PATH="$python_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
```

`PYO3_PYTHON` selects the Python used for building, `PYTHONHOME` locates its base standard library, and `LD_LIBRARY_PATH` locates its shared library on Linux.
The example programs read `SPACY_SITE_PACKAGES` and pass its directories to `Config::new()`.
For other platforms, use the corresponding virtual-environment layout and shared-library loader configuration described in [PyO3's build guide](https://pyo3.rs/v0.30.1/building-and-distribution).

## Usage

### Extract named entities from a text file

This program loads `en_core_web_sm`, reads a UTF-8 file, and writes each entity's text and label as tab-separated columns.
It uses the default `entities` feature.
After preparing the Python environment above, save the following as `src/main.rs` in a binary project with the dependency shown under Installation:

```rust
use spacy_sys::{Config, Python};

use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let input = env::args_os().nth(1).ok_or("usage: spacy-entities INPUT")?;
    let text = fs::read_to_string(PathBuf::from(input))?;

    let package_dirs = env::var_os("SPACY_SITE_PACKAGES")
        .ok_or("set SPACY_SITE_PACKAGES to the Python package directories")?;

    let nlp = Config::new(env::split_paths(&package_dirs)).load("en_core_web_sm")?;

    let mut out = io::BufWriter::new(io::stdout().lock());

    Python::attach(|py| -> Result<(), Box<dyn Error>> {
        let doc = nlp.process(py, &text)?;

        for entity in doc.entities()? {
            let entity = entity?;

            writeln!(out, "{}\t{}", entity.text()?, entity.label()?)?;
        }

        Ok(())
    })?;

    out.flush()?;

    Ok(())
}
```

Create an input file and run the program in the same shell:

```sh
printf '%s\n' 'Alice works at Microsoft in London.' > article.txt
cargo run --quiet -- article.txt
```

Output:

```text
Alice	PERSON
Microsoft	ORG
London	GPE
```

The output depends on the pipeline; the example above uses `en_core_web_sm` 3.8.0.
To analyze different text, pass another UTF-8 file.
To use another installed pipeline, change the name passed to `Config::load()`.

`Language::make_doc()` runs only the tokenizer, while `Language::process()` also runs the active pipeline components.
`Language::pipe()` processes a batch lazily in input order.
For sentence iteration, the pipeline must provide boundaries through a parser, sentence recognizer, or sentencizer.
A blank pipeline created with `Language::blank()` needs a sentence component before calling `Doc::sentences()`.
Python exceptions are returned as `Error` (`pyo3::PyErr`); fallible iterators terminate after an error or exhaustion.

`Token::attribute()` reads the attribute specified by `TokenAttribute`: `Pos` for `pos_`, `Tag` for `tag_`, `Lemma` for `lemma_`, and `Morph` for `str(morph)`.
The `parsing` feature adds `Dep` (`dep_`); `entities` adds `EntIob` (`ent_iob_`) and `EntType` (`ent_type_`).
Available annotations depend on the loaded pipeline.

## Package and model locations

The example passes the iterator from `env::split_paths()` directly to `Config::new()` before loading the pipeline.
Applications can supply directories from any existing Python environment compatible with the linked interpreter.
Selecting a virtual environment with `PYO3_PYTHON` alone does not guarantee that its site-packages will be searched.

For model packages, include the installation directory containing the package and its distribution metadata.
`Config::load_path()` and `Language::load_path()` load a model directory instead of an installed package name; its Python dependencies and registered components must still be importable.

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
