use std::{collections::BTreeMap, path::PathBuf, process::Command};

use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::Deserialize;

use crate::{command, version};

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    manifest_path: PathBuf,
    publish: Option<Vec<String>>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    rename: Option<String>,
    req: String,
    kind: Option<String>,
    path: Option<PathBuf>,
    registry: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Crate {
    pub name: String,
    pub version: Version,
    pub manifest_path: PathBuf,
    /// Actual package names, including optional, target, and build dependencies.
    pub dependencies: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct Workspace {
    pub root: PathBuf,
    pub target_directory: PathBuf,
    pub crates: BTreeMap<String, Crate>,
}

impl Workspace {
    pub fn load(manifest_path: Option<PathBuf>) -> Result<Self> {
        let mut cmd = Command::new("cargo");
        cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
        if let Some(path) = manifest_path {
            cmd.arg("--manifest-path").arg(path);
        }
        let metadata: Metadata = serde_json::from_slice(&command::output(&mut cmd)?.stdout)
            .context("invalid cargo metadata output")?;
        Self::from_metadata(metadata)
    }

    fn from_metadata(metadata: Metadata) -> Result<Self> {
        let packages: Vec<_> = metadata
            .packages
            .iter()
            .filter(|p| metadata.workspace_members.contains(&p.id))
            .collect();
        let mut crates = BTreeMap::new();
        for package in &packages {
            if package.publish.as_ref().is_some_and(|registries| {
                !registries.iter().any(|registry| registry == "crates-io")
            }) {
                continue;
            }
            let mut dependencies = Vec::new();
            for dep in &package.dependencies {
                // Dev-only edges do not constrain cargo publish's build order.
                if dep.kind.as_deref() == Some("dev") {
                    continue;
                }
                let Some(path) = &dep.path else { continue };
                ensure!(
                    dep.registry.is_none(),
                    "{}: path dependency {} uses an alternate registry",
                    package.name,
                    dep.name
                );
                let dependency = packages
                    .iter()
                    .find(|p| {
                        p.name == dep.name && p.manifest_path.parent() == Some(path.as_path())
                    })
                    .with_context(|| {
                        format!(
                            "{}: path dependency {} must be a workspace member",
                            package.name, dep.name
                        )
                    })?;
                ensure!(
                    dependency
                        .publish
                        .as_ref()
                        .is_none_or(|registries| { registries.iter().any(|r| r == "crates-io") }),
                    "{} depends on {}, which cannot be published to crates.io",
                    package.name,
                    dep.name
                );
                let current = version::parse(&dependency.version)?;
                ensure!(
                    version::requires_current(&dep.req, &current)?,
                    "{}: dependency {} ({}) requires {}, but the workspace uses {}; \
                     update its minimum requirement to {} and bump the consumer's version \
                     if its published package changes",
                    package.name,
                    dep.rename.as_deref().unwrap_or(&dep.name),
                    dep.name,
                    dep.req,
                    current,
                    current
                );
                dependencies.push(dep.name.clone());
            }
            dependencies.sort();
            dependencies.dedup();
            crates.insert(
                package.name.clone(),
                Crate {
                    name: package.name.clone(),
                    version: version::parse(&package.version)?,
                    manifest_path: package.manifest_path.clone(),
                    dependencies,
                },
            );
        }
        ensure!(
            !crates.is_empty(),
            "no crates.io-publishable workspace packages"
        );
        Ok(Self {
            root: metadata.workspace_root,
            target_directory: metadata.target_directory,
            crates,
        })
    }

    pub fn package(&self) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.root).args([
            "package",
            "--no-verify",
            "--allow-dirty",
            "--registry",
            "crates-io",
        ]);
        // Package together: Cargo stages unpublished workspace dependencies when
        // creating normalized manifests and registry-based lockfiles.
        for name in self.crates.keys() {
            cmd.arg("--package").arg(name);
        }
        eprintln!(
            "Preparing {} package archives (without compiling)...",
            self.crates.len()
        );
        command::output(&mut cmd)?;
        Ok(())
    }

    pub fn ensure_clean(&self) -> Result<()> {
        let output = command::output(Command::new("git").current_dir(&self.root).args([
            "status",
            "--porcelain",
            "--untracked-files=normal",
        ]))?;
        ensure!(
            output.stdout.is_empty(),
            "publishing requires a clean Git checkout; commit or remove pending changes first:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(requirement: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            format!(
                r#"
[workspace]
members = ["core", "consumer", "helper"]
resolver = "3"
[workspace.package]
edition = "2024"
license = "MIT"
[workspace.dependencies]
renamed = {{ package = "fixture-core", path = "core", version = "{requirement}" }}
"#
            ),
        )
        .unwrap();
        for (name, extra) in [
            ("core", ""),
            (
                "consumer",
                "[target.'cfg(unix)'.dependencies]\nrenamed = { workspace = true, optional = true }\n[build-dependencies]\nfixture-core = { path = '../core', version = '0.1.2' }\n[dev-dependencies]\nfixture-helper = { path = '../helper' }\n",
            ),
            ("helper", ""),
        ] {
            let root = dir.path().join(name);
            std::fs::create_dir_all(root.join("src")).unwrap();
            let private = if name == "helper" {
                "publish = false"
            } else {
                ""
            };
            std::fs::write(
                root.join("Cargo.toml"),
                format!(
                    r#"
[package]
name = "fixture-{name}"
version = "0.1.2"
edition.workspace = true
license.workspace = true
{private}
{extra}
"#
                ),
            )
            .unwrap();
            std::fs::write(root.join("src/lib.rs"), "pub fn example() {}\n").unwrap();
        }
        dir
    }

    #[test]
    fn cargo_metadata_resolves_aliases_optional_target_and_build_edges() {
        let fixture = fixture("0.1.2");
        let workspace = Workspace::load(Some(fixture.path().join("Cargo.toml"))).unwrap();
        assert_eq!(workspace.crates.len(), 2);
        assert_eq!(
            workspace.crates["fixture-consumer"].dependencies,
            ["fixture-core"]
        );
        workspace.package().unwrap();
        let bytes = std::fs::read(
            workspace
                .target_directory
                .join("package/fixture-consumer-0.1.2.crate"),
        )
        .unwrap();
        // Packaging a workspace works even though neither crate exists on crates.io.
        crate::archive::Archive::read(
            &bytes,
            "fixture-consumer",
            &version::parse("0.1.2").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn stale_inherited_requirement_is_rejected_before_packaging() {
        let fixture = fixture("0.1.1");
        let error = Workspace::load(Some(fixture.path().join("Cargo.toml")))
            .unwrap_err()
            .to_string();
        assert!(error.contains("renamed (fixture-core)"), "{error}");
        assert!(
            error.contains("update its minimum requirement to 0.1.2"),
            "{error}"
        );
    }

    #[test]
    fn dependency_release_requires_consumer_manifest_and_version_updates() {
        use crate::{
            archive::Archive,
            registry::{PublishedVersion, Registry},
            release,
        };

        struct Snapshot(BTreeMap<String, Archive>);
        impl Registry for Snapshot {
            fn versions(&self, name: &str) -> Result<Vec<PublishedVersion>> {
                Ok(if self.0.contains_key(name) {
                    vec![PublishedVersion {
                        version: "0.1.2".parse().unwrap(),
                        checksum: String::new(),
                        yanked: false,
                    }]
                } else {
                    vec![]
                })
            }
            fn archive(&self, name: &str, _: &PublishedVersion) -> Result<Archive> {
                Ok(self.0[name].clone())
            }
        }

        let fixture = fixture("0.1.2");
        let manifest = fixture.path().join("Cargo.toml");
        let original = release::prepare(
            &Workspace::load(Some(manifest.clone())).unwrap(),
            &Snapshot(BTreeMap::new()),
        )
        .unwrap();
        let registry = Snapshot(original.archives);
        // Bump core and both dependency requirements, but forget the consumer's
        // version. The consumer's own source file has not changed at all.
        for path in [manifest.clone(), fixture.path().join("core/Cargo.toml")] {
            let content = std::fs::read_to_string(&path)
                .unwrap()
                .replace("0.1.2", "0.1.3");
            std::fs::write(path, content).unwrap();
        }
        let consumer = fixture.path().join("consumer/Cargo.toml");
        let content = std::fs::read_to_string(&consumer)
            .unwrap()
            .replace("version = '0.1.2'", "version = '0.1.3'");
        std::fs::write(&consumer, content).unwrap();
        let error = release::prepare(&Workspace::load(Some(manifest.clone())).unwrap(), &registry)
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains("fixture-consumer 0.1.2:")
                && error.contains("Cargo.toml")
                && error.contains("bump the package version"),
            "{error}"
        );

        let content = std::fs::read_to_string(&consumer)
            .unwrap()
            .replace("0.1.2", "0.1.3");
        std::fs::write(&consumer, content).unwrap();
        let plan = release::prepare(&Workspace::load(Some(manifest)).unwrap(), &registry).unwrap();
        assert_eq!(
            plan.pending
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["fixture-core", "fixture-consumer"]
        );
    }
}
