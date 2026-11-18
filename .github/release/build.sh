#!/usr/bin/env bash
set -euo pipefail

: "${TARGET:?}" "${PYTHON_VERSION:?}" "${PYTHON_TAG:?}"
export PYO3_PYTHON="${PYO3_PYTHON:-$(command -v python)}"
export PYTHONNOUSERSITE=1
unset PYTHONHOME PYTHONPATH

# Compilation and linking require CPython, but no installed Python packages.
"$PYO3_PYTHON" - "$PYTHON_VERSION" "$PYTHON_TAG" <<'PY'
import sys
import sysconfig

version, tag = sys.argv[1:]
assert sys.implementation.name == "cpython"
assert f"{sys.version_info.major}.{sys.version_info.minor}" == version
assert f"cp{sys.version_info.major}{sys.version_info.minor}" == tag
assert not sysconfig.get_config_var("Py_GIL_DISABLED"), "Use GIL-enabled CPython"
assert sysconfig.get_config_var("Py_ENABLE_SHARED") or sysconfig.get_config_var("PYTHONFRAMEWORK"), "Shared CPython is required"
PY

cargo build --locked --target "$TARGET" -p lita-tokenizers-cli --profile release-minified

package="lita-tokenizers-cli-$TARGET-$PYTHON_TAG"
target_dir="${CARGO_TARGET_DIR:-target}"
mkdir -p "$package/license/"{kytea,lita-tokenizers-cli,mecab}
cp "$target_dir/$TARGET/release-minified/lita-tokenizers-cli" "$package/"
cp LICENSE-APACHE LICENSE-MIT "$package/license/lita-tokenizers-cli/"
cp crates/kytea-sys/kytea-src/COPYING "$package/license/kytea/"
cp crates/mecab-sys/mecab-src/{COPYING,BSD} "$package/license/mecab/"
tar -czf "$package.tar.gz" "$package"
