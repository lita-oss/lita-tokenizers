# Developing the spaCy crates

The integration suite covers both the general `spacy-sys` wrapper and the
`lita-tokenizers-spacy` adapter. Tests use a dedicated environment rather than
installing into the developer's normal Python environment. They fail when
required environment settings or packages are missing; they do not skip cases.

## Local development and CI

Install Rust, [uv](https://docs.astral.sh/uv/), and a runnable shared-library
CPython. CPython 3.12 is the baseline used by the container recipes. Select a real
interpreter executable rather than a shell alias or a launcher requiring extra
arguments:

```sh
PYO3_PYTHON=/usr/bin/python3 sh crates/spacy-sys/tests/run.sh --locked
PYO3_PYTHON=/usr/bin/python3 sh crates/spacy-sys/tests/run.sh --locked --no-default-features
```

Run these commands from any directory. The script resolves the workspace root,
copies `tests/assets/uv/pyproject.toml` and `uv.lock` into `target/spacy-tests/project`,
and installs the locked test group, including spaCy 3.8.18 and `en_core_web_sm`
3.8.0. Python downloads are disabled. The initial package setup needs network
access; subsequent runs reuse the environment and its uv cache. Rust's `--locked`
and uv's `--locked` protect different dependency graphs; the script always uses
uv's lockfile, and forwards arguments to `cargo test` for both crates.
The Python version range in the test manifest controls dependency resolution;
it does not describe a tested Python/platform matrix.

`SPACY_TEST_ROOT` overrides the test cache location. Otherwise the script uses
`CARGO_TARGET_DIR/spacy-tests`, or `target/spacy-tests` when no target directory is
specified. `UV` can select a uv executable. The script queries the test Python for
its package directories, base installation, and library directory, configures the
launcher environment, and exports:

- `SPACY_TEST_ROOT`: writable generated-fixture directory.
- `SPACY_TEST_PYTHON`: test environment's Python executable.
- `SPACY_TEST_SITE_PACKAGES`: package directories separated by the platform's path separator.
- `PYO3_PYTHON`: the same test Python used to prepare the packages.

It also sets `PYO3_ENVIRONMENT_SIGNATURE` from the base interpreter's version and
configuration, so recreating a venv at the same path with a different Python
invalidates PyO3's cached build configuration.

Each crate writes generated models into its own subdirectory of the test root.
Test helpers read these settings and supply explicit search paths to `Config`.
Python's module cache is process-wide, so import-source scenarios use fresh test
processes. Normal tests never download anything; provisioning happens before the
test processes start. `cargo clean` removes the default test cache.

CI should run the default and disabled-feature suites, then formatting, lints,
and documentation checks:

```sh
cargo fmt -p spacy-sys -p lita-tokenizers-spacy -- --check
PYO3_PYTHON=/usr/bin/python3 cargo clippy -p spacy-sys -p lita-tokenizers-spacy --all-targets --all-features -- -D warnings
PYO3_PYTHON=/usr/bin/python3 RUSTDOCFLAGS='-D warnings' cargo doc -p spacy-sys -p lita-tokenizers-spacy --all-features --no-deps
```

The test script includes doctests. Examples requiring Python packages are marked
`no_run`: they are compiled, while integration tests execute the corresponding
operations with provisioned dependencies. These checks do not establish support
for Python versions or platforms that have not been exercised.

A workspace-wide build includes the spaCy backend crates, facade, and CLI.
Jobs without Python must exclude both backend crates and disable default features,
for example `cargo check --workspace --no-default-features --exclude spacy-sys --exclude lita-tokenizers-spacy`.

## Containers

From the workspace root:

```sh
docker build -f crates/lita-tokenizers-spacy/tests/Dockerfile.debian -t lita-spacy:debian .
docker run --rm lita-spacy:debian

docker build -f crates/lita-tokenizers-spacy/tests/Dockerfile.alpine -t lita-spacy:alpine .
docker run --rm lita-spacy:alpine
```

Both recipes provision uv-managed CPython 3.12 before invoking the test harness.
The Alpine recipe disables static CRT linking so Python can load native extension
modules. It also depends on native wheels or build prerequisites being available
for the pinned Python packages on musl; a Debian result alone does not validate it.

## Updating dependencies

The uv project and lockfile are test fixtures, not a public runtime environment
API. Update them together with uv, run both suites, and review annotation
expectations when changing spaCy or the trained model.
Before adding a spaCy version to the supported range, run the integration suite
against that version and update the README's supported-version section.
Application package versions, model choices, and cache directories remain
application policy.
