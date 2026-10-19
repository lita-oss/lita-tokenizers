use std::{collections::BTreeMap, io::Read};

use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;

/// Compare shipped inputs, not gzip timestamps or Cargo's generated bookkeeping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Archive(BTreeMap<String, File>);

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
        Ok(Self(files))
    }

    pub fn differences(&self, other: &Self) -> Vec<String> {
        self.0
            .keys()
            .chain(other.0.keys())
            .filter(|key| self.0.get(*key) != other.0.get(*key))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    #[cfg(test)]
    pub fn fixture(content: &str) -> Self {
        Self(BTreeMap::from([(
            "src/lib.rs".into(),
            File {
                content: content.as_bytes().to_vec(),
                executable: false,
            },
        )]))
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
}
