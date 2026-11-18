#!/usr/bin/env bash
set -euo pipefail

: "${TARGET:?}" "${PYTHON_TAG:?}"
python="${PYO3_PYTHON:-$(command -v python)}"
export PYTHONNOUSERSITE=1
unset PYTHONHOME PYTHONPATH

smoke_dir=$(mktemp -d)
trap 'rm -rf "$smoke_dir"' EXIT

# spaCy is a runtime test dependency only. Keep it out of the build interpreter.
"$python" -m venv "$smoke_dir/venv"
python="$smoke_dir/venv/bin/python"
"$python" -m pip install 'spacy==3.8.18'
python_libdir=$("$python" -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')
export LD_LIBRARY_PATH="$python_libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$python_libdir${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export PYTHONPATH
PYTHONPATH=$("$python" -I -c 'import os, sys; print(os.pathsep.join(sys.path))')

package="lita-tokenizers-cli-$TARGET-$PYTHON_TAG"
tar -xzf "$package.tar.gz" -C "$smoke_dir"
bin="$smoke_dir/$package/lita-tokenizers-cli"
"$bin" --version
printf '%s\n' 'The quick brown fox jumps over the lazy dog.' \
    | "$bin" spacy --charset utf8 --model-path crates/spacy-sys/tests/models/lita-en-minimal \
        --tag pos - > "$smoke_dir/actual.txt"
printf 'The/DET\tquick/ADJ\tbrown/ADJ\tfox/NOUN\tjumps/VERB\tover/ADP\tthe/DET\tlazy/ADJ\tdog/NOUN\t./PUNCT\n' \
    > "$smoke_dir/expected.txt"
diff -u "$smoke_dir/expected.txt" "$smoke_dir/actual.txt"
