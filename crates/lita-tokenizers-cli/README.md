# `lita-tokenizers-cli`

[![crates.io](https://img.shields.io/crates/v/lita-tokenizers-cli.svg)](https://crates.io/crates/lita-tokenizers-cli)
[![License](https://img.shields.io/crates/l/lita-tokenizers-cli.svg)](https://crates.io/crates/lita-tokenizers-cli)

CLI for LiTA tokenizers.

This part is effectively no more than the compatibility layer between the command line and the `lita-tokenizers` crate.


## Usage

First, download the pre-compiled binary from the [Release page](https://github.com/lita-oss/lita-tokenizers/releases/latest).
Or, build the binary from the source:

```
$ cargo install lita-tokenizers-cli
```

The manual build may require the C++ compiler and a [shared CPython installation](../spacy-sys#build-and-runtime-requirements).

Then you can tokenize as:

```
$ echo "すもももももももものうち" | lita-tokenizers-cli kytea -m KYTEA_MODEL_PATH -
すもも/名詞/すもも	も/助詞/も	も/助詞/も	も/助詞/も	も/助詞/も	も/助詞/も	もの/名詞/もの	うち/名詞/うち
```


### KyTea tokenizer

```
$ lita-tokenizers-cli kytea -c CHARSET -m MODEL -t TAG -o OUTPUT INPUT
```

- `CHARSET` (*optional*, defaults to `dynamic`): character encodings. Acceptable values are `utf8`, `shift_jis`, `euc-jp`, and `dynamic`.
- `MODEL` (*required*): the path to the pre-trained KyTea model.
- `TAG` (*optional*, defaults to "as-is"): re-ordering of tags. Comma-separated indices. Read the documentation of `lita_tokenizers::kytea::TagIndex`.
- `OUTPUT` (*optional*, defaults to stdout): the path of the output file or directory.
- `INPUT` (*required*): the path of the input file or directory, or `"-"` for stdin.


### MeCab tokenizer

```
$ lita-tokenizers-cli mecab -c CHARSET -d DICT -r DICTRC -t TAG -o OUTPUT INPUT
```

- `CHARSET` (*optional*, defaults to `dynamic`): character encodings. Acceptable values are `utf8`, `shift_jis`, `euc-jp`, and `dynamic`.
- `DICT` (*required*): the path of the directory that contains the MeCab model files.
- `DICTRC` (*required*): the path of `dictrc` file.
- `TAG` (*optional*, defaults to "as-is"): re-ordering of tags. Comma-separated indices. Read the documentation of `lita_tokenizers::kytea::TagIndex`.
- `OUTPUT` (*optional*, defaults to stdout): the path of the output file or directory.
- `INPUT` (*required*): the path of the input file or directory, or `"-"` for stdin.


### spaCy tokenizer

```
$ lita-tokenizers-cli spacy -c CHARSET -m MODEL --site-packages SITE_PACKAGES -t TAG -o OUTPUT INPUT
```

- `CHARSET` (*optional*, defaults to `dynamic`): character encodings. Acceptable values are `utf8`, `shift_jis`, `euc-jp`, and `dynamic`.
- `MODEL` (*required unless `MODEL_PATH` is provided*): the installed spaCy model package or the path to the pre-trained spaCy model. The model must provide sentence boundaries.
- `MODEL_PATH` (*optional*, `--model-path`): the path of the directory that contains the spaCy model files. Cannot be used with `MODEL`.
- `SITE_PACKAGES` (*optional*): the path of a Python package directory. Can be specified more than once.
- `TAG` (*optional*, defaults to `pos,tag,lemma,morph`): selection and ordering of tags. Comma-separated attributes `pos`, `tag`, `lemma`, and `morph`. An empty string omits tags; missing annotations are empty strings. Read the documentation of `lita_tokenizers::spacy::TokenAttribute`.
- `OUTPUT` (*optional*, defaults to stdout): the path of the output file or directory.
- `INPUT` (*required*): the path of the input file or directory, or `"-"` for stdin.


### Whitespace tokenizer

```
$ lita-tokenizers-cli whitespace -c CHARSET -o OUTPUT INPUT
```

- `CHARSET` (*optional*, defaults to `dynamic`): character encodings. Acceptable values are `utf8`, `shift_jis`, `euc-jp`, and `dynamic`.
- `OUTPUT` (*optional*, defaults to stdout): the path of the output file or directory.
- `INPUT` (*required*): the path of the input file or directory, or `"-"` for stdin.
