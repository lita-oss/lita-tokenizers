use std::{io::Read, time::Duration};

use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::archive::Archive;

#[derive(Clone, Debug)]
pub(crate) struct PublishedVersion {
    pub version: Version,
    pub checksum: String,
    pub yanked: bool,
}

pub(crate) trait Registry {
    fn versions(&self, name: &str) -> Result<Vec<PublishedVersion>>;
    fn archive(&self, name: &str, version: &PublishedVersion) -> Result<Archive>;
}

pub(crate) struct CratesIo {
    client: reqwest::blocking::Client,
}

impl CratesIo {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .https_only(true)
            .user_agent(concat!("lita-tokenizers-xtask/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }

    fn get(&self, url: &str) -> Result<Option<Vec<u8>>> {
        let response = self
            .client
            .get(url)
            .header("Cache-Control", "no-cache")
            .send()
            .with_context(|| format!("GET {url}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("GET {url}"))?;
        // Bound memory use even if a server sends an unexpectedly large response.
        let mut bytes = Vec::new();
        response
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("reading {url}"))?;
        ensure!(bytes.len() <= 64 * 1024 * 1024, "response too large: {url}");
        Ok(Some(bytes))
    }
}

fn index_path(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

fn parse_index(bytes: &[u8]) -> Result<Vec<PublishedVersion>> {
    #[derive(Deserialize)]
    struct Entry {
        vers: String,
        cksum: String,
        yanked: bool,
    }
    let versions: Vec<_> = std::str::from_utf8(bytes)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let entry: Entry = serde_json::from_str(line)?;
            Ok(PublishedVersion {
                version: Version::parse(&entry.vers)?,
                checksum: entry.cksum,
                yanked: entry.yanked,
            })
        })
        .collect::<Result<_>>()?;
    ensure!(
        !versions.is_empty(),
        "registry returned an empty index document"
    );
    Ok(versions)
}

impl Registry for CratesIo {
    fn versions(&self, name: &str) -> Result<Vec<PublishedVersion>> {
        let url = format!("https://index.crates.io/{}", index_path(name));
        let Some(bytes) = self.get(&url)? else {
            return Ok(Vec::new());
        };
        parse_index(&bytes).with_context(|| format!("reading registry versions of {name}"))
    }

    fn archive(&self, name: &str, version: &PublishedVersion) -> Result<Archive> {
        let url = format!(
            "https://static.crates.io/crates/{name}/{name}-{}.crate",
            version.version
        );
        let bytes = self
            .get(&url)?
            .with_context(|| format!("published archive not found: {url}"))?;
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == version.checksum,
            "checksum mismatch for {name} {}",
            version.version
        );
        Archive::read(&bytes, name, &version.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_index_paths() {
        for (name, path) in [
            ("a", "1/a"),
            ("ab", "2/ab"),
            ("AbC", "3/a/abc"),
            ("demo", "de/mo/demo"),
        ] {
            assert_eq!(index_path(name), path);
        }
    }

    #[test]
    fn bad_index_data_fails_instead_of_treating_package_as_new() {
        assert!(parse_index(b"server error").is_err());
        assert!(parse_index(b"").is_err());
        let versions = parse_index(
            br#"{"vers":"0.10.0","cksum":"abc","yanked":true}
{"vers":"0.9.0","cksum":"def","yanked":false}"#,
        )
        .unwrap();
        assert!(versions[0].version > versions[1].version);
        assert!(versions[0].yanked);
    }
}
