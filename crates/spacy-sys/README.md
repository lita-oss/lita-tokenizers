# spacy-sys

[![crates.io](https://img.shields.io/crates/v/spacy-sys.svg)](https://crates.io/crates/spacy-sys)
[![docs.rs](https://docs.rs/spacy-sys/badge.svg)](https://docs.rs/spacy-sys)
[![License](https://img.shields.io/crates/l/spacy-sys.svg)](https://crates.io/crates/spacy-sys)

**Rust FFI bindings for [spaCy](https://spacy.io/).**

`spacy-sys` provides typed, safe wrappers to the spaCy (Python) library through PyO3.
This crate embeds a CPython interpreter. **You need to install Python and spaCy on your system to use this crate.**


**Only prediction mode is supported now.**
If you want to train a model, still you have to use spaCy.

---


## Prerequisites

This crate **does not** bundle the spaCy model files. To actually use spaCy for text analysis, you must download (or train by yourself) a pre-trained model.

You will also need a shared-library build of CPython installed on your system so PyO3 can embed the interpreter, and spaCy 3.8.18 and the model's Python dependencies installed for that interpreter.
Set `PYO3_PYTHON` to the interpreter executable and configure the shared-library loader path, such as `LD_LIBRARY_PATH` on Linux.
To configure the interpreter behavior, check the documentation of [PyO3](https://pyo3.rs/v0.30.1/building-and-distribution).
The tests also require uv to install spaCy into `.tmp/site-packages`.



## Installation

Add `spacy-sys` to your `Cargo.toml`:

```toml
[dependencies]
spacy-sys = "0.1.0"
```


## Usage

Basic usage for English morphological analysis:

```rust
use spacy_sys::{Config, Error, Python};

fn main() -> Result<(), Error> {
    let model = Config::new(["/path/to/your/site-packages"])
        .load_path("/path/to/your/model")?;

    Python::attach(|py| {
        let doc = model.process(py, "The quick brown fox jumps over the lazy dog.")?;

        for token in doc.tokens()? {
            let token = token?;
            let surface = token.text()?;
            let pos = token.pos()?;

            println!("{surface}: {pos}");
        }

        Ok(())
    })
}
```

## Tests

We have tests against the minimal model that can parse the sentence `The quick brown fox jumps over the lazy dog.`.

The model is located at `tests/models/lita-en-minimal/`, and the input corpus and training configuration are located at `tests/models/input/`.
With spaCy 3.8.18 installed for Python 3.15.2, run these commands from this crate's directory to train a model from that corpus using the [spaCy CLI](https://spacy.io/api/cli#train).
The training and evaluation inputs use the same one-sentence corpus; the trained model is saved to `.tmp/training/model-best/`.

```text
$ mkdir -p .tmp
$ python -m spacy convert ./tests/models/input/train.conllu ./.tmp --converter conllu --n-sents 1
$ python -m spacy init fill-config ./tests/models/input/config.cfg ./.tmp/config.cfg
$ python -m spacy train ./.tmp/config.cfg --output ./.tmp/training --paths.train ./.tmp/train.spacy --paths.dev ./.tmp/train.spacy
$ python -c 'import spacy; nlp = spacy.load("./.tmp/training/model-best"); print("\n".join(f"{t.text}\t{t.pos_}\t{t.dep_}\t{t.head.i}" for t in nlp("The quick brown fox jumps over the lazy dog.")))'
The	DET	det	3
quick	ADJ	amod	3
brown	ADJ	amod	3
fox	NOUN	nsubj	4
jumps	VERB	ROOT	4
over	ADP	case	8
the	DET	det	8
lazy	ADJ	amod	8
dog	NOUN	obl	4
.	PUNCT	punct	4
```
