# Building CLI releases

The tag-triggered `../workflows/deploy.yml` workflow builds every platform with
the CLI's default features, including spaCy. `matrix.py` supplies both the build
matrix and the expected release assets, so packaging and release verification
use the same target and Python combinations.

## Supported Python versions

The release range is the intersection of the supported CPython minor versions
for spaCy and PyO3, bounded below by 3.11 and above by 3.15. The currently reviewed
dependencies are:

- [spaCy 3.8.18](https://pypi.org/project/spacy/3.8.18/): CPython 3.10–3.15.
- [PyO3 0.30.1](https://github.com/PyO3/pyo3/blob/v0.30.1/pyo3-ffi/build.rs):
  CPython 3.9–3.15. The next interpreter version's experimental support is excluded.

This selects CPython 3.11–3.15, using ordinary GIL-enabled builds. With four
platforms, there are 20 archives. When updating these dependencies, review the
support bounds in `matrix.py` together with Cargo.lock. The spaCy version in
`smoke.sh` selects the runtime test dependency; it does not affect compilation
or linking. Python patch releases follow the providers' selected minor-version
releases.

## Build environments and validation

GNU/Linux and macOS builds use native Rust and `actions/setup-python` for each
selected interpreter. `build.sh` selects that interpreter explicitly for PyO3
without importing or installing spaCy. Only shared CPython is required to compile
and link the artifacts.

musl builds run natively inside `python:<minor>-alpine3.23`, whose CPython has a
shared library built against musl. `musl.Dockerfile` installs Rust, a C/C++
toolchain, and the build requirements for spaCy dependencies that lack compatible
wheels. Rust's static CRT feature is disabled for this target so shared CPython
and Python extension modules can load. An Ubuntu/glibc Python installation must
not be used to link the musl artifact.

`build.sh` verifies the interpreter ABI, builds with the committed Cargo lockfile,
and packages the binary and licenses. After packaging, `smoke.sh` creates a
temporary virtual environment and installs spaCy there solely for runtime tests.
It derives the library and module search paths from that environment, extracts
the archive, and checks `--version` and exact CLI token/POS output using the
checked-in minimal English model. The release job requires every expected
archive and rejects unexpected or empty files.

To reproduce a musl build from the repository root (substitute another selected
minor version and its matching tag as needed):

```bash
docker build --platform linux/amd64 \
  --build-arg PYTHON_VERSION=3.14 \
  --tag lita-release:cp314 --file .github/release/musl.Dockerfile .github/release
docker run --rm --platform linux/amd64 --volume "$PWD:/work" \
  --user "$(id -u):$(id -g)" --env CARGO_HOME=/tmp/cargo \
  --env TARGET=x86_64-unknown-linux-musl \
  --env PYTHON_VERSION=3.14 --env PYTHON_TAG=cp314 \
  lita-release:cp314 bash -c \
    'bash .github/release/build.sh && bash .github/release/smoke.sh'
```

The container writes build products and the release archive into the mounted
checkout. To inspect the generated plan without building:

```bash
python3 .github/release/matrix.py
```
