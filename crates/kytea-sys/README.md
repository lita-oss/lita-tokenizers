# kytea-sys

[![crates.io](https://img.shields.io/crates/v/kytea-sys.svg)](https://crates.io/crates/kytea-sys)
[![docs.rs](https://docs.rs/kytea-sys/badge.svg)](https://docs.rs/kytea-sys)
[![License](https://img.shields.io/crates/l/kytea-sys.svg)](https://crates.io/crates/kytea-sys)

**Rust FFI bindings for [KyTea](http://www.phontron.com/kytea/).**

`kytea-sys` provides raw, unsafe FFI bindings to the KyTea (C++) library.
This crate is designed to be completely self-contained: it bundles the KyTea source code, compiles it directly using the `cc` crate, and links it statically. **You do not need to install KyTea on your system to use this crate.**


**Only prediction mode is supported now.**
If you want to train a model via KyTea, still you have to use the C++ library.

---


## Prerequisites

This crate **does not** bundle the KyTea model file, as it is quite large. To actually use KyTea for text analysis, you must download (or train by yourself) a pre-trained model.

You will also need a standard C++ compiler (like `g++` or `clang++`) installed on your system so the `cc` crate can build the bundled C++ source.
To configure the compiler behavior, check the documentation of [cc](https://crates.io/crates/cc).



## Installation

Add `kytea-sys` to your `Cargo.toml`:

```toml
[dependencies]
kytea-sys = "0.1.0"
```


## Usage

Basic usage for Japanese morphological analysis:

```rust
use kytea_sys::{CorpusFormat, KyTea, StringStream};

fn main() {
    let model_path = c"/path/to/your/kytea/model";
    let mut model = KyTea::new();

    model.read_model(model_path).unwrap();

    model.config().set_training(false).set_input_format(CorpusFormat::Raw);

    let mut input = {
        let mut ss = StringStream::new();
        ss.push("すもももももももものうち\n");
        ss
    };
    let mut output = StringStream::new();

    let mut ctx = model.context(&mut input, &mut output).unwrap();
    while ctx.predict().unwrap().is_continue() {}

    println!("{}", String::from_utf8_lossy(output.as_bytes()));
}
```

You can use `Fstream`s for input/output as well:

```rust
use kytea_sys::{CorpusFormat, KyTea, StringStream, Fstream};

fn main() {
    let model_path = c"/path/to/your/kytea/model";
    let mut model = KyTea::new();

    model.read_model(model_path).unwrap();

    model.config().set_training(false).set_input_format(CorpusFormat::Raw);

    let mut input = Fstream::open(c"/path/to/input").unwrap();
    let mut output = Fstream::create(c"/path/to/output").unwrap();

    let mut ctx = model.context(&mut input, &mut output).unwrap();
    while ctx.predict().unwrap().is_continue() {}
    output.flush().unwrap();

    println!("{}", std::fs::read_to_string("/path/to/output").unwrap());
}
```

## Tests

We have tests against the minimal models that can parse the sentence `すもももももももものうち．`.

The models, located at `tests/models/`, were trained as

```
$ echo 'すもも/名詞/すもも も/助詞/も もも/名詞/もも も/助詞/も もも/名詞/もも の/助詞/の うち/名詞/うち ．/補助記号/。' >train.txt
$ train-kytea -full train.txt -charw 4 -model ./tests/models/model.bin
$ train-kytea -full train.txt -charw 4 -modtext -model ./tests/models/model.txt
```

so that

```
$ echo "すもももももももものうち．" | kytea -model ./tests/models/model.bin
すもも/名詞/すもも も/助詞/も もも/名詞/もも も/助詞/も もも/名詞/もも の/助詞/の うち/名詞/うち ．/補助記号/。
$ echo "すもももももももものうち．" | kytea -model ./tests/models/model.txt
すもも/名詞/すもも も/助詞/も もも/名詞/もも も/助詞/も もも/名詞/もも の/助詞/の うち/名詞/うち ．/補助記号/。
```
