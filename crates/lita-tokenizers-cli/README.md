# `lita-tokenizers-cli`

CLI for LiTA tokenizers.

This part is effectively no more than the compatibility layer between the command line and the `lita-tokenizers` crate.


## Usage

First, download the pre-compiled binary from the [Release page](https://github.com/lita-oss/lita-tokenizers/releases/latest).
Or, build the binary from the source:

```
$ cargo install lita-tokenizers-cli
```

The manual build may require the C++ compiler.

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


### Whitespace tokenizer

```
$ lita-tokenizers-cli whitespace -c CHARSET -o OUTPUT INPUT
```

- `CHARSET` (*optional*, defaults to `dynamic`): character encodings. Acceptable values are `utf8`, `shift_jis`, `euc-jp`, and `dynamic`.
- `OUTPUT` (*optional*, defaults to stdout): the path of the output file or directory.
- `INPUT` (*required*): the path of the input file or directory, or `"-"` for stdin.
