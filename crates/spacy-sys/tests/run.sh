#!/bin/sh
# Provision test-only packages, then build and run both crates with the same Python.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
cd "$repo"
python=${PYO3_PYTHON:-python3}
uv=${UV:-uv}
unset PYTHONHOME PYTHONPATH
command -v "$uv" >/dev/null 2>&1 || {
    echo 'spaCy tests require uv; install it before running tests/run.sh' >&2
    exit 1
}
"$python" -c 'import sys; assert sys.implementation.name == "cpython", "tests require CPython"' || {
    echo 'select a runnable CPython interpreter with PYO3_PYTHON' >&2
    exit 1
}

root=${SPACY_TEST_ROOT:-${CARGO_TARGET_DIR:-target}/spacy-tests}
mkdir -p "$root"
root=$(CDPATH= cd -- "$root" && pwd)
project="$root/project"
mkdir -p "$project"
cp crates/spacy-sys/tests/assets/uv/pyproject.toml "$project/pyproject.toml"
cp crates/spacy-sys/tests/assets/uv/uv.lock "$project/uv.lock"
export UV_CACHE_DIR="$root/uv-cache"
export UV_PYTHON_DOWNLOADS=never
unset UV_PROJECT_ENVIRONMENT
# The packages must be installed for the selected base interpreter, never a downloaded one.
"$uv" sync --locked --group test --no-python-downloads --python "$python" --project "$project"

case $(uname -s) in
    MINGW*|MSYS*|CYGWIN*) test_python="$project/.venv/Scripts/python.exe" ;;
    *) test_python="$project/.venv/bin/python" ;;
esac
base=$("$test_python" -c 'import sys; print(sys.base_prefix)')
libdir=$("$test_python" -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR") or "")')
site_packages=$("$test_python" -c 'import os, sysconfig; print(os.pathsep.join(dict.fromkeys(sysconfig.get_path(p) for p in ("purelib", "platlib"))))')
export PYO3_PYTHON="$test_python"
# Recreating the venv can change its base interpreter without changing this path.
export PYO3_ENVIRONMENT_SIGNATURE
PYO3_ENVIRONMENT_SIGNATURE=$("$test_python" -c 'import sys, sysconfig; print((sys.version, sys.base_prefix, sys.base_exec_prefix, sysconfig.get_config_var("SOABI")))')
export PYTHONHOME="$base"
unset PYTHONPATH
export SPACY_TEST_ROOT="$root"
export SPACY_TEST_PYTHON="$test_python"
export SPACY_TEST_SITE_PACKAGES="$site_packages"
if [ -n "$libdir" ]; then
    case $(uname -s) in
        Darwin) export DYLD_LIBRARY_PATH="$libdir${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" ;;
        *) export LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ;;
    esac
fi
exec cargo test -p spacy-sys -p lita-tokenizers-spacy "$@"
