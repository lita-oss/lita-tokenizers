# Release planning and publishing

```sh
cargo run -p xtask -- plan
cargo run -p xtask -- publish
```

Both commands independently validate the entire workspace against crates.io.
There is no saved plan to go stale: `publish` recomputes it before any upload.
Use the same reviewed commit and Rust/Cargo toolchain locally and in CI.
This tool uses Cargo's multi-package packaging support (tested with Cargo 1.99).

## Version policy

- Packages with `publish = false`, including `xtask`, are excluded. Packages
  restricted to other registries are also excluded.
- Package versions must be `major.minor.patch`. A new crate is scheduled for its
  first publication. A version above its highest published stable version is
  scheduled for publication. Older local versions are rejected; this tool assumes
  a single release line, not maintenance releases below a newer published version.
- An existing version is skipped **only when its packaged inputs match the
  registry archive**. Changed, added, or deleted packaged files at an existing
  version produce an error requiring a version bump. Yanked versions cannot be
  reused, and registry/network errors fail validation.
- Every normal/build path dependency must be a crates.io-publishable workspace
  member. Its requirement must accept the workspace version and declare a full
  minimum version at least that new. For example, after updating a dependency to
  `0.1.2`, `"0.1.1"` is rejected even though its implicit caret requirement accepts
  `0.1.2`. Update it to `"0.1.2"` (or an equivalent explicit lower bound).
- Updating that requirement changes each consumer's resolved manifest. Already
  published consumers therefore also need new package versions. This is a
  deliberately conservative policy: it propagates releases through consumers
  rather than trying to infer which source changes need newer dependency APIs.
  Optional and target-specific dependencies participate, as do build dependencies.
  Dev-only dependencies do not add publishing-order edges.
- Before reporting validation failures, the planner propagates required version
  bumps through every published consumer in dependency order. For example, a
  required bump of `lita-tokenizers-kytea` also reports requirement/version updates
  for `lita-tokenizers` and then `lita-tokenizers-cli`, even while their current
  packages still match crates.io. Each affected package is listed once, with all
  dependency causes. An already-unpublished consumer needs its requirements
  updated but can keep its planned version; it does not trigger another artificial
  version bump downstream.
- The tool checks that versions were advanced, not whether a change deserves a
  patch, minor, or major bump. Choose the appropriate SemVer bump during review.

## How the comparison works

`cargo metadata --no-deps` supplies package identities and dependency requirements,
including aliases and workspace inheritance. `cargo package --no-verify` prepares
all publishable members together so unpublished dependencies can be staged by
Cargo. Planning does not compile native bindings or upload anything, but it needs
network access and writes ordinary Cargo artifacts under `target/` (and may update
Cargo's caches/lockfiles). Keep the workspace lockfile current before releasing.

The planner queries the crates.io sparse index. For existing local versions it
downloads the published archive and verifies its SHA-256 checksum against the
index. Comparison includes vendored native sources, headers, readmes, and other
packaged files. The generated, normalized `Cargo.toml` is compared semantically,
so inherited package metadata and dependency requirements are included. Comments
and key order do not matter. Executable file permissions are also compared.
Cargo's `Cargo.toml.orig` and `.cargo_vcs_info.json` are excluded. A library's
generated package `Cargo.lock` is excluded because consumers resolve their own
dependencies. For packages with binary targets, the lockfile is compared
semantically too: `cargo install --locked` uses it, so lockfile-only binary changes
to dependency versions or sources require a version bump. Checksums for the exact
workspace package names and versions are ignored during lockfile comparison:
Cargo's temporary packaging registry gives rebuilt workspace archives different
hashes from the published archives. External dependency checksums are still
compared, and downloaded crate archives are always verified against the crates.io
index checksum. An explicit version bump still schedules publication even when
sources match.

## Publishing

`publish` requires a clean Git checkout, including no untracked files. It uses
`std::process::Command` to run `cargo publish --registry crates-io --manifest-path
<package manifest>` in deterministic dependency order. Cargo's normal package
build verification stays enabled. Credentials come from Cargo's usual
configuration or `CARGO_REGISTRY_TOKEN`; the tool neither stores nor prints them.

After each upload it checks registry visibility and package contents before
continuing to consumers. `--wait-timeout <seconds>` controls the visibility wait
(default: 120; each HTTPS request also has a 30-second timeout). Failure stops the
release. Rerun the same command to resume: matching versions from a partial run
are checked and skipped. An unexpected existing version with different contents
is an error, never an automatic skip. If Cargo reports failure after an upload,
the tool checks the registry before declaring the step failed.

Publishing is idempotent for the same workspace inputs: after a successful run,
another `publish` recomputes the plan, verifies all published versions against the
newly generated archives, prints `Nothing to publish.`, and exits successfully
without invoking `cargo publish`. It does not require a saved local release record
or publishing credentials for this no-op. Editing packaged inputs at an existing
version still fails validation rather than being silently skipped.

`publish --dry-run` performs the same validation and clean-checkout check, then
runs one multi-package `cargo publish --dry-run` so Cargo can stage dependencies
that are not published yet. `--manifest-path <path>` selects a workspace from a
different directory for either subcommand.

For CI, first run your tests and platform/feature checks on the release commit,
then run `plan` and `publish`. Set up crates.io Trusted Publishing separately and
pass its temporary token as `CARGO_REGISTRY_TOKEN`. Serialize publishing runs for
the repository. The tool does not configure authentication, create tags/releases,
edit manifests, or perform version bumps for you.

HTTPS, gzip, hashing, manifest parsing, and archive reading use Rust libraries;
no `curl`, shell scripts, OpenSSL installation, or release helper binaries are
needed at runtime. The rustls crypto dependency (`ring`) needs a C toolchain when
building `xtask`; the workspace's native bindings already require C++ tooling.

## Development

```sh
cargo test -p xtask
cargo clippy -p xtask --all-targets -- -D warnings
cargo fmt -p xtask -- --check
```

Tests use an in-memory registry and temporary Cargo workspaces; they never upload
to crates.io. For fully offline testing with cached dependencies, set
`CARGO_NET_OFFLINE=true` so the nested Cargo commands also stay offline.
