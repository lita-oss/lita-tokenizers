# Developing spacy-sys

The tests assume uv and shared CPython are already installed, and `PYO3_PYTHON`
and the shared-library loader path, such as `LD_LIBRARY_PATH`, are already set.
Tests use that interpreter; they do not install Python or change its runtime
configuration.

## Tests

```sh
cargo test -p spacy-sys --locked
cargo test -p spacy-sys --locked --no-default-features
```

The `tokenize()` test in `src/tests.rs` loads `tests/models/lita-en-minimal`,
relative to `CARGO_MANIFEST_DIR`. This model was generated using Python 3.15.2
and parses “The quick brown fox jumps over the lazy dog.” with UD POS tags and
dependency relations. Test models are excluded from the Cargo package.

The test helper installs spaCy 3.8.18 and its dependencies into
`CARGO_MANIFEST_DIR/.tmp/site-packages` using blocking `uv pip install` with the
configured `PYO3_PYTHON`. It waits for `.tmp/site-packages.lock` to be released,
then reuses an existing `.tmp/site-packages/spacy` directory. If spaCy is absent,
it acquires the lock, installs the package, and releases the lock. The lock file
can remain on disk after release; its existence does not mean it is locked.

The first installation needs network access. Subsequent runs reuse the package
directory. Install failures fail the test. With no tests running, remove
`.tmp/site-packages` before switching Python ABIs or changing the pinned spaCy
version.

## Checks

```sh
cargo fmt -p spacy-sys -- --check
cargo clippy -p spacy-sys --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p spacy-sys --all-features --no-deps
```

Examples requiring Python packages are marked `no_run` and are compile-checked.
The tokenization test checks the pangram's tokens, POS tags, and dependency parse.

A workspace-wide build includes the spaCy backend crates, facade, and CLI.
Jobs without Python must exclude both backend crates and disable default features,
for example `cargo check --workspace --no-default-features --exclude spacy-sys --exclude lita-tokenizers-spacy`.
