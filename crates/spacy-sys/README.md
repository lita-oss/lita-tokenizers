# spacy-sys

[![crates.io](https://img.shields.io/crates/v/spacy-sys.svg)](https://crates.io/crates/spacy-sys)
[![docs.rs](https://docs.rs/spacy-sys/badge.svg)](https://docs.rs/spacy-sys)
[![License](https://img.shields.io/crates/l/spacy-sys.svg)](https://crates.io/crates/spacy-sys)

**A general Rust wrapper for [spaCy](https://spacy.io/), using PyO3 to embed CPython.**

This crate is independent of LiTA and `lita-tokenizers-core`.
For the LiTA tokenization traits, use [lita-tokenizers-spacy](../lita-tokenizers-spacy).

The wrapper provides pipelines, documents, sentences, tokens, and common annotations.
Training, GPU management, and the full spaCy API are outside its typed interface; `as_python()` exposes the underlying object for additional operations through the re-exported `python` (PyO3) crate.

---


## Build and runtime requirements

Install a shared-library build of CPython before building this crate or a binary that depends on it.
PyO3 discovers Python from the environment; select it explicitly with an interpreter executable:

```sh
PYO3_PYTHON=/usr/bin/python3 cargo build -p spacy-sys
```

Distribution-provided Python may also require its development/linker package (for example, `python3-dev` on Debian/Ubuntu).
Having only the versioned runtime library, such as `libpython3.12.so.1.0`, may leave the linker unable to find `-lpython3.12`.

The build checks that Python uses a shared library.
It never installs Python, spaCy, or models.
Ordinary builds do not need the `spacy` package.
Python selection also applies to `cargo check`, rust-analyzer, and downstream builds.
Cross-compilation additionally needs the target Python configuration described in [PyO3's build guide](https://pyo3.rs/v0.30.1/building-and-distribution).

At runtime, the application must provide the corresponding Python shared library, standard library, and native dependencies.
Python's ABI, architecture, and platform must match the installed extension modules.
`PYO3_PYTHON` selects the build configuration; changing it at runtime does not change the linked Python.
Embedding Python does not normally launch a Python subprocess.

The OS must find libpython before the Rust program starts.
Configure the loader or application launcher accordingly; for example, Linux deployments may need `LD_LIBRARY_PATH`.
A program linked against libpython cannot install a missing libpython from its own `main()`.
Shipping a binary requires a compatible external Python installation or a distribution that includes the runtime and its startup configuration.
A virtual environment alone does not contain the base Python standard library.
If a launcher sets `PYTHONHOME`, it must refer to that base installation, not merely the virtual environment.

The integration environment pins spaCy 3.8.18 and the English 3.8.0 model on CPython 3.12.
The test project's Python range is a dependency-resolution range, not a claim that every Python/platform combination has been validated.

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

`Language::load("en_core_web_sm")` loads an installed trained pipeline; `Language::load_path(path)` loads a pipeline directory.
Loading never downloads models.
A blank language provides tokenization but no trained annotations or sentence boundaries; add a sentencizer or load a pipeline supplying boundaries before calling `Doc::sentences()`.

`Language::make_doc()` runs only the tokenizer.
`Language::pipe()` exposes spaCy's batch-processing iterator.
Python exceptions from construction, processing, and iteration are returned as the concrete `Error` (`pyo3::PyErr`).
A fallible iterator terminates after its first error, as well as after exhaustion.

For code that selects string-valued annotations at runtime, `TokenAttribute` identifies spaCy attributes directly: `Pos` (`pos_`), `Tag` (`tag_`), `Lemma` (`lemma_`), and `Morph` (`str(morph)`).
`Token::attribute(kind)` reads the chosen attribute.
The `parsing` feature adds `Dep` (`dep_`); `entities` adds `EntIob` (`ent_iob_`) and `EntType` (`ent_type_`).
This enum is not a token-tag ordering or a list of linguistic labels such as `NOUN`: it specifies which attribute to read.

## Package and model locations

Applications choose where to install spaCy and models.
Existing installations from pip, uv, or other tooling work when their packages are searchable.
Selecting a virtual environment with `PYO3_PYTHON` does not by itself guarantee that the embedded interpreter searches that environment's site-packages.

Use `Config` to add package directories explicitly:

```rust,no_run
use spacy_sys::{Config, Error};
use std::path::Path;

let paths = [Path::new("/opt/my-app/venv/lib/python3.12/site-packages")];
let nlp = Config::new(&paths).load("en_core_web_sm")?;
# Ok::<(), Error>(())
```

Query `sysconfig.get_path('purelib')` and `sysconfig.get_path('platlib')` with the Python used to prepare the environment instead of guessing their layout.
For models installed as packages, include the directory containing the package and its distribution metadata.
A model directory still needs any Python dependencies and registered components it uses.

Each configured path is passed to `site.addsitedir()` before importing spaCy.
Paths supplement the existing search path and `.pth` files are processed, including their executable lines.
This does not isolate environments or override an already imported package.
Search paths and imported modules are process-wide; the additions persist after a `Config` or `Language` is dropped.
Configure paths before the first spaCy import and manage one compatible environment per process.

## Ownership and costs

`Language` owns a Python reference.
`Doc`, `Span`, and `Token` hold Python references bound to the lifetime of an attached interpreter; Python keeps their backing data alive.
They do not borrow the input string or the `Language`.
Keep Python handles within `Python::attach()` and clone and unbind the underlying Python handle when it is needed beyond that attachment, for example `nlp.as_python(py).clone().unbind()`.

`Config` borrows its path slice without copying paths or allocating storage.
Typed wrappers add no Rust heap allocation or dynamic dispatch.
Iterators advance lazily without collecting their results.
Python calls, Python reference counting, and conversion across the FFI boundary still have their normal costs.
String getters copy into Rust-owned strings at that boundary; `Token::text_ref()` returns a Python string handle for borrowed access through PyO3.
No zero-copy claim is made for Python string conversion or serialization.

## Feature flags

All optional features are enabled by default and add no dependencies:

- `entities`: Document entities and span labels.
- `parsing`: Token dependency labels, heads, and children.
- `serialization`: Pipeline/document saving and document byte serialization.

Disable them with `default-features = false`.
Tokenization, sentence iteration, POS, fine POS, lemmas, morphology, and basic pipeline operations remain available.
Features control the Rust interface; they do not install Python packages or enable spaCy pipeline components.

For development, integration tests, and containers, see [CONTRIBUTING.md](CONTRIBUTING.md).
