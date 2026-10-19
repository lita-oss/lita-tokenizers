use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;

/// Compare shipped inputs, not gzip timestamps or Cargo's generated bookkeeping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Archive {
    files: BTreeMap<String, File>,
    workspace_packages: BTreeSet<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct File {
    content: Vec<u8>,
    executable: bool,
}

impl Archive {
    pub fn read(bytes: &[u8], name: &str, version: &semver::Version) -> Result<Self> {
        let mut archive = tar::Archive::new(GzDecoder::new(bytes));
        let prefix = format!("{name}-{version}/");
        let mut files = BTreeMap::new();
        let mut has_binaries = false;
        for entry in archive.entries()? {
            let mut entry = entry?;
            ensure!(
                entry.header().entry_type().is_file(),
                "unexpected non-file archive entry"
            );
            let path = entry.path()?.to_string_lossy().into_owned();
            let relative = path
                .strip_prefix(&prefix)
                .with_context(|| format!("unexpected archive path: {path}"))?;
            if matches!(relative, "Cargo.toml.orig" | ".cargo_vcs_info.json") {
                continue;
            }
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            if matches!(relative, "Cargo.toml" | "Cargo.lock") {
                // Cargo resolves workspace inheritance and path dependencies in
                // this manifest. Ignore formatting and key order, not semantics.
                let manifest: toml::Value = toml::from_str(std::str::from_utf8(&content)?)?;
                if relative == "Cargo.toml" {
                    has_binaries = manifest
                        .get("bin")
                        .and_then(toml::Value::as_array)
                        .is_some_and(|bins| !bins.is_empty());
                }
                content = serde_json::to_vec(&manifest)?;
            }
            let file = File {
                content,
                executable: entry.header().mode()? & 0o111 != 0,
            };
            ensure!(
                files.insert(relative.to_owned(), file).is_none(),
                "duplicate archive path: {path}"
            );
        }
        ensure!(
            files.contains_key("Cargo.toml"),
            "archive has no Cargo.toml"
        );
        // A library consumer resolves its own lockfile. A binary installed with
        // --locked uses this one, so dependency-only binary changes need a bump.
        if !has_binaries {
            files.remove("Cargo.lock");
        }
        Ok(Self {
            files,
            workspace_packages: BTreeSet::new(),
        })
    }

    pub fn with_workspace_packages<'a>(
        mut self,
        packages: impl IntoIterator<Item = (&'a str, &'a semver::Version)>,
    ) -> Self {
        self.workspace_packages = packages
            .into_iter()
            .map(|(name, version)| (name.to_owned(), version.to_string()))
            .collect();
        self
    }

    pub fn differences(&self, other: &Self) -> Vec<String> {
        self.files
            .keys()
            .chain(other.files.keys())
            .filter(|key| !self.same_file(key, other))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn same_file(&self, path: &str, other: &Self) -> bool {
        let (Some(local), Some(remote)) = (self.files.get(path), other.files.get(path)) else {
            return false;
        };
        if local == remote {
            return true;
        }
        if path != "Cargo.lock" || local.executable != remote.executable {
            return false;
        }
        // A workspace package archive is rebuilt in Cargo's staging registry.
        // Its hash can differ from the published archive, even when its shipped
        // inputs match (e.g. different VCS bookkeeping). Compare the lockfile's
        // resolution, but not these temporary hashes. External checksums stay exact.
        let (Ok(mut local), Ok(mut remote)) = (
            serde_json::from_slice::<serde_json::Value>(&local.content),
            serde_json::from_slice::<serde_json::Value>(&remote.content),
        ) else {
            return false;
        };
        let workspace_packages = self
            .workspace_packages
            .union(&other.workspace_packages)
            .cloned()
            .collect();
        Self::normalize_workspace_checksums(&mut local, &workspace_packages);
        Self::normalize_workspace_checksums(&mut remote, &workspace_packages);
        local == remote
    }

    fn normalize_workspace_checksums(
        lock: &mut serde_json::Value,
        workspace_packages: &BTreeSet<(String, String)>,
    ) {
        let Some(packages) = lock
            .get_mut("package")
            .and_then(serde_json::Value::as_array_mut)
        else {
            return;
        };
        for package in packages {
            if package.get("source").and_then(serde_json::Value::as_str)
                != Some("registry+https://github.com/rust-lang/crates.io-index")
            {
                continue;
            }
            let (Some(name), Some(version)) = (
                package.get("name").and_then(serde_json::Value::as_str),
                package.get("version").and_then(serde_json::Value::as_str),
            ) else {
                continue;
            };
            if workspace_packages.contains(&(name.to_owned(), version.to_owned()))
                && let Some(package) = package.as_object_mut()
            {
                package.remove("checksum");
            }
        }
    }

    #[cfg(test)]
    pub fn fixture(content: &str) -> Self {
        Self {
            files: BTreeMap::from([(
                "src/lib.rs".into(),
                File {
                    content: content.as_bytes().to_vec(),
                    executable: false,
                },
            )]),
            workspace_packages: BTreeSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};

    fn pack(files: &[(&str, &str)]) -> Vec<u8> {
        let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(
                &mut header,
                format!("demo-1.0.0/{path}"),
                content.as_bytes(),
            )
            .unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn compares_resolved_manifests_and_shipped_files() {
        let version = semver::Version::new(1, 0, 0);
        let original = pack(&[
            (
                "Cargo.toml",
                "[package]\nname = 'demo'\nversion = '1.0.0'\n",
            ),
            ("src/lib.rs", "original"),
            ("Cargo.lock", "version = 3"),
            ("Cargo.toml.orig", "inherited workspace fields"),
        ]);
        let identical = pack(&[
            ("Cargo.toml", "[package]\nversion='1.0.0'\nname='demo'\n"),
            ("src/lib.rs", "original"),
            ("Cargo.lock", "version = 4"),
            (".cargo_vcs_info.json", "different commit"),
        ]);
        let original = Archive::read(&original, "demo", &version).unwrap();
        assert_eq!(
            original,
            Archive::read(&identical, "demo", &version).unwrap()
        );
        let changed = pack(&[
            ("Cargo.toml", "[package]\nname='demo'\nversion='1.0.0'\n"),
            ("src/lib.rs", "changed"),
            ("native/header.h", "new header"),
        ]);
        assert_eq!(
            original.differences(&Archive::read(&changed, "demo", &version).unwrap()),
            ["native/header.h", "src/lib.rs"]
        );
    }

    #[test]
    fn binary_lockfile_changes_require_a_new_version() {
        let manifest =
            "[package]\nname='demo'\nversion='1.0.0'\n[[bin]]\nname='demo'\npath='src/main.rs'\n";
        let original = pack(&[("Cargo.toml", manifest), ("Cargo.lock", "version = 3")]);
        let changed = pack(&[("Cargo.toml", manifest), ("Cargo.lock", "version = 4")]);
        let version = semver::Version::new(1, 0, 0);
        let original = Archive::read(&original, "demo", &version).unwrap();
        let changed = Archive::read(&changed, "demo", &version).unwrap();
        assert_eq!(original.differences(&changed), ["Cargo.lock"]);
    }

    fn binary_archive(lock: &str) -> Archive {
        let manifest =
            "[package]\nname='demo'\nversion='1.0.0'\n[[bin]]\nname='demo'\npath='src/main.rs'\n";
        Archive::read(
            &pack(&[("Cargo.toml", manifest), ("Cargo.lock", lock)]),
            "demo",
            &semver::Version::new(1, 0, 0),
        )
        .unwrap()
    }

    fn staged_lock() -> String {
        format!(
            r#"version = 4
[[package]]
name = "workspace-core"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "{}"
dependencies = ["external"]
[[package]]
name = "external"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "{}"
"#,
            "a".repeat(64),
            "b".repeat(64)
        )
    }

    #[test]
    fn staging_and_published_workspace_checksums_are_equivalent() {
        let staged = staged_lock();
        let published = staged.replace(&"a".repeat(64), &"c".repeat(64));
        let local = binary_archive(&staged);
        let remote = binary_archive(&published);
        assert_eq!(local.differences(&remote), ["Cargo.lock"]);
        let version = semver::Version::new(0, 1, 0);
        let local = local.with_workspace_packages([("workspace-core", &version)]);
        assert!(local.differences(&remote).is_empty());
        assert!(remote.differences(&local).is_empty());
    }

    #[test]
    fn workspace_checksum_normalization_preserves_other_lockfile_changes() {
        let staged = staged_lock();
        let version = semver::Version::new(0, 1, 0);
        let local = binary_archive(&staged).with_workspace_packages([("workspace-core", &version)]);
        for changed in [
            staged.replace(&"b".repeat(64), &"c".repeat(64)), // External checksum.
            staged.replace("version = \"0.1.0\"", "version = \"0.1.1\""),
            staged.replace("dependencies = [\"external\"]", "dependencies = []"),
            staged.replace(
                "source = \"registry+https://github.com/rust-lang/crates.io-index\"",
                "source = \"registry+https://other.invalid/index\"",
            ),
        ] {
            assert_eq!(local.differences(&binary_archive(&changed)), ["Cargo.lock"]);
        }
        let published = staged.replace(&"a".repeat(64), &"c".repeat(64));
        let wrong_version = semver::Version::new(0, 1, 1);
        let wrong_scope =
            binary_archive(&staged).with_workspace_packages([("workspace-core", &wrong_version)]);
        assert_eq!(
            wrong_scope.differences(&binary_archive(&published)),
            ["Cargo.lock"]
        );
        let alternate = staged.replace(
            "github.com/rust-lang/crates.io-index",
            "other.invalid/index",
        );
        let local =
            binary_archive(&alternate).with_workspace_packages([("workspace-core", &version)]);
        let remote = binary_archive(&alternate.replace(&"a".repeat(64), &"c".repeat(64)));
        assert_eq!(local.differences(&remote), ["Cargo.lock"]);
    }
}
