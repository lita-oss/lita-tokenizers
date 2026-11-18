"""Generate the release matrix and the exact expected asset names."""

import json

# Reviewed for spaCy 3.8.18 and PyO3 0.30.1. See README.md before updating.
SPACY_PYTHON = (10, 15)
PYO3_PYTHON = (9, 15)

platforms = (
    ("x86_64-unknown-linux-musl", "ubuntu-24.04"),
    ("x86_64-unknown-linux-gnu", "ubuntu-24.04"),
    ("x86_64-apple-darwin", "macos-15-intel"),
    ("aarch64-apple-darwin", "macos-15"),
)
minimum = max(SPACY_PYTHON[0], PYO3_PYTHON[0], 11)
maximum = min(SPACY_PYTHON[1], PYO3_PYTHON[1], 15)
if minimum > maximum:
    raise ValueError("spaCy and PyO3 have no supported release Python versions")

entries = [
    {
        "target": target,
        "os": os,
        "python": f"3.{minor}",
        "python_tag": f"cp3{minor}",
    }
    for target, os in platforms
    for minor in range(minimum, maximum + 1)
]
packages = [
    f"lita-tokenizers-cli-{entry['target']}-{entry['python_tag']}.tar.gz"
    for entry in entries
]
print("matrix=" + json.dumps({"include": entries}, separators=(",", ":")))
print("packages=" + json.dumps(packages, separators=(",", ":")))
