use std::{
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};

use crate::{
    registry::{CratesIo, Registry},
    release::{self, ReleasePlan},
    workspace::{Crate, Workspace},
};

#[derive(Debug)]
pub struct Arg {
    pub manifest_path: Option<PathBuf>,
    pub dry_run: bool,
    pub wait_timeout: Duration,
}

impl Default for Arg {
    fn default() -> Self {
        Self {
            manifest_path: None,
            dry_run: false,
            wait_timeout: Duration::from_secs(120),
        }
    }
}

pub fn run(arg: Arg) -> Result<()> {
    ensure!(!arg.wait_timeout.is_zero(), "wait timeout must be positive");
    let workspace = Workspace::load(arg.manifest_path)?;
    let registry = CratesIo::new()?;
    let plan = release::prepare(&workspace, &registry)?;
    plan.print();
    if plan.pending.is_empty() {
        return Ok(());
    }
    workspace.ensure_clean()?;
    if arg.dry_run {
        // A batch dry run lets Cargo stage unpublished dependencies locally.
        // Separate per-crate dry runs could not resolve those dependencies yet.
        let mut cmd = publish_command(&workspace);
        cmd.arg("--dry-run");
        for package in &plan.pending {
            cmd.arg("--package").arg(&package.name);
        }
        ensure!(cmd.status()?.success(), "cargo publish --dry-run failed");
        return Ok(());
    }
    execute(&plan, &registry, arg.wait_timeout, |package| {
        workspace.ensure_clean()?;
        let status = publish_command(&workspace)
            .arg("--manifest-path")
            .arg(&package.manifest_path)
            .status()
            .with_context(|| format!("starting cargo publish for {}", package.name))?;
        Ok(status.success())
    })
}

fn execute(
    plan: &ReleasePlan,
    registry: &impl Registry,
    wait_timeout: Duration,
    mut upload: impl FnMut(&Crate) -> Result<bool>,
) -> Result<()> {
    for package in &plan.pending {
        let local = &plan.archives[&package.name];
        if release::is_published(package, local, registry)? {
            println!("Already published: {} {}", package.name, package.version);
            continue;
        }
        println!("Publishing {} {}...", package.name, package.version);
        let succeeded = upload(package)?;
        // A nonzero exit may mean the upload succeeded but index polling timed
        // out. Check the registry before deciding that the release failed.
        let start = Instant::now();
        loop {
            if release::is_published(package, local, registry)? {
                println!("Published {} {}", package.name, package.version);
                break;
            }
            ensure!(
                succeeded,
                "cargo publish failed for {} {}; rerun publish after investigating (completed crates will be checked and skipped)",
                package.name,
                package.version
            );
            ensure!(
                start.elapsed() < wait_timeout,
                "{} {} is not visible in the registry after {:?}; rerun publish to resume",
                package.name,
                package.version,
                wait_timeout
            );
            thread::sleep(Duration::from_secs(2).min(wait_timeout.saturating_sub(start.elapsed())));
        }
    }
    Ok(())
}

fn publish_command(workspace: &Workspace) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&workspace.root)
        .args(["publish", "--registry", "crates-io"]);
    cmd
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap, io::Read};

    use super::*;
    use crate::{archive::Archive, registry::PublishedVersion};

    struct RegistryFixture(RefCell<BTreeMap<String, Archive>>);

    impl Registry for RegistryFixture {
        fn versions(&self, name: &str) -> Result<Vec<PublishedVersion>> {
            Ok(if self.0.borrow().contains_key(name) {
                vec![PublishedVersion {
                    version: "1.0.0".parse().unwrap(),
                    checksum: String::new(),
                    yanked: false,
                }]
            } else {
                Vec::new()
            })
        }

        fn archive(&self, name: &str, _: &PublishedVersion) -> Result<Archive> {
            Ok(self.0.borrow()[name].clone())
        }
    }

    fn plan() -> ReleasePlan {
        ReleasePlan {
            pending: ["core", "api"]
                .into_iter()
                .map(|name| Crate {
                    name: name.into(),
                    version: "1.0.0".parse().unwrap(),
                    manifest_path: format!("{name}/Cargo.toml").into(),
                    dependencies: if name == "api" {
                        vec!["core".into()]
                    } else {
                        vec![]
                    },
                })
                .collect(),
            already_published: vec![],
            archives: ["core", "api"]
                .into_iter()
                .map(|name| (name.into(), Archive::fixture(name)))
                .collect(),
        }
    }

    fn uploaded_archive(workspace: &Workspace, package: &Crate) -> (Archive, usize) {
        use flate2::{Compression, read::GzDecoder, write::GzEncoder};

        let path = workspace
            .target_directory
            .join("package")
            .join(format!("{}-{}.crate", package.name, package.version));
        let bytes = std::fs::read(path).unwrap();
        let mut input = tar::Archive::new(GzDecoder::new(bytes.as_slice()));
        let mut output = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        let mut changed_checksums = 0;
        for entry in input.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().into_owned();
            let mut content = Vec::new();
            entry.read_to_end(&mut content).unwrap();
            if path.ends_with("Cargo.lock") {
                let mut lock: toml::Value =
                    toml::from_str(std::str::from_utf8(&content).unwrap()).unwrap();
                for entry in lock.get_mut("package").unwrap().as_array_mut().unwrap() {
                    let name = entry["name"].as_str().unwrap();
                    if workspace.crates.get(name).is_some_and(|p| {
                        Some(p.version.to_string().as_str()) == entry["version"].as_str()
                    }) && entry.get("checksum").is_some()
                    {
                        // Real cargo publish packages crates separately against
                        // registry hashes, unlike the planning staging registry.
                        entry["checksum"] = toml::Value::String("f".repeat(64));
                        changed_checksums += 1;
                    }
                }
                content = toml::to_string(&lock).unwrap().into_bytes();
            }
            let mut header = entry.header().clone();
            header.set_size(content.len() as u64);
            header.set_cksum();
            output
                .append_data(&mut header, path, content.as_slice())
                .unwrap();
        }
        let bytes = output.into_inner().unwrap().finish().unwrap();
        (
            Archive::read(&bytes, &package.name, &package.version).unwrap(),
            changed_checksums,
        )
    }

    #[test]
    fn publishes_in_order_and_resumes_without_repeating_uploads() {
        let plan = plan();
        let registry = RegistryFixture(RefCell::new(BTreeMap::new()));
        let mut uploaded = Vec::new();
        execute(&plan, &registry, Duration::ZERO, |package| {
            if package.name == "api" {
                assert!(registry.0.borrow().contains_key("core"));
            }
            uploaded.push(package.name.clone());
            registry
                .0
                .borrow_mut()
                .insert(package.name.clone(), plan.archives[&package.name].clone());
            Ok(true)
        })
        .unwrap();
        assert_eq!(uploaded, ["core", "api"]);
        execute(&plan, &registry, Duration::ZERO, |_| {
            panic!("must skip existing versions")
        })
        .unwrap();
    }

    #[test]
    fn successful_publish_is_idempotent_after_reloading_and_repackaging_workspace() {
        let fixture = tempfile::tempdir().unwrap();
        let manifest = fixture.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[workspace]\nmembers = ['core', 'cli']\nresolver = '3'\n",
        )
        .unwrap();
        for name in ["core", "cli"] {
            let root = fixture.path().join(name);
            std::fs::create_dir_all(root.join("src")).unwrap();
            let dependency = if name == "cli" {
                "[dependencies]\nbackend = { package = 'idempotency-fixture-core', path = '../core', version = '1.0.0' }\n"
            } else {
                ""
            };
            std::fs::write(root.join("Cargo.toml"), format!(
                "[package]\nname = 'idempotency-fixture-{name}'\nversion = '1.0.0'\nedition = '2024'\nlicense = 'MIT'\n{dependency}"
            )).unwrap();
            let (source, content) = if name == "cli" {
                ("main.rs", "fn main() {}\n")
            } else {
                ("lib.rs", "pub fn example() {}\n")
            };
            std::fs::write(root.join("src").join(source), content).unwrap();
        }
        let registry = RegistryFixture(RefCell::new(BTreeMap::new()));
        let workspace = Workspace::load(Some(manifest.clone())).unwrap();
        let first = release::prepare(&workspace, &registry).unwrap();
        let mut uploads = Vec::new();
        let mut changed_checksums = 0;
        execute(&first, &registry, Duration::ZERO, |package| {
            uploads.push(package.name.clone());
            let (archive, changed) = uploaded_archive(&workspace, package);
            changed_checksums += changed;
            registry
                .0
                .borrow_mut()
                .insert(package.name.clone(), archive);
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            uploads,
            ["idempotency-fixture-core", "idempotency-fixture-cli"]
        );
        assert!(
            changed_checksums > 0,
            "must exercise staging versus published checksums"
        );
        let published = registry.0.borrow().clone();

        // Model a separate invocation: reload metadata, regenerate all archives
        // (including the CLI's lockfile), and rerun the complete validation.
        let workspace = Workspace::load(Some(manifest)).unwrap();
        let second = release::prepare(&workspace, &registry).unwrap();
        assert!(second.pending.is_empty());
        assert_eq!(second.already_published.len(), 2);
        assert_eq!(first.archives, second.archives);
        execute(&second, &registry, Duration::ZERO, |_| {
            panic!("second publish must not upload")
        })
        .unwrap();
        assert_eq!(*registry.0.borrow(), published);

        // Idempotency must never hide a conflicting edit at an existing version.
        std::fs::write(
            fixture.path().join("cli/src/main.rs"),
            "fn main() { println!(\"changed\"); }\n",
        )
        .unwrap();
        let error = release::prepare(&workspace, &registry)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("bump the package version"), "{error}");
        assert_eq!(*registry.0.borrow(), published);
    }

    #[test]
    fn stops_on_failure_before_publishing_consumers() {
        let plan = plan();
        let registry = RegistryFixture(RefCell::new(BTreeMap::new()));
        let mut uploaded = Vec::new();
        assert!(
            execute(&plan, &registry, Duration::ZERO, |package| {
                uploaded.push(package.name.clone());
                Ok(false)
            })
            .is_err()
        );
        assert_eq!(uploaded, ["core"]);
    }

    #[test]
    fn checks_registry_after_cargo_reports_failure() {
        let plan = plan();
        let registry = RegistryFixture(RefCell::new(BTreeMap::new()));
        execute(&plan, &registry, Duration::ZERO, |package| {
            registry
                .0
                .borrow_mut()
                .insert(package.name.clone(), plan.archives[&package.name].clone());
            Ok(false) // Upload succeeded, but Cargo's index polling failed.
        })
        .unwrap();
    }

    #[test]
    fn successful_command_without_registry_visibility_stops_at_timeout() {
        let plan = plan();
        let registry = RegistryFixture(RefCell::new(BTreeMap::new()));
        assert!(
            execute(&plan, &registry, Duration::ZERO, |_| Ok(true))
                .unwrap_err()
                .to_string()
                .contains("not visible")
        );
    }
}
