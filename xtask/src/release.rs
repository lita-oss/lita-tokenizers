use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};

use crate::{
    archive::Archive,
    registry::Registry,
    workspace::{Crate, Workspace},
};

pub(crate) struct ReleasePlan {
    pub pending: Vec<Crate>,
    pub already_published: Vec<String>,
    pub archives: BTreeMap<String, Archive>,
}

impl ReleasePlan {
    pub fn print(&self) {
        for name in &self.already_published {
            println!("UNCHANGED {name} (matches crates.io)");
        }
        if self.pending.is_empty() {
            println!("Nothing to publish.");
        } else {
            println!("Publish order:");
            for (i, package) in self.pending.iter().enumerate() {
                println!("  {}. {} {}", i + 1, package.name, package.version);
            }
        }
    }
}

pub(crate) fn prepare(workspace: &Workspace, registry: &impl Registry) -> Result<ReleasePlan> {
    let order = dependency_order(&workspace.crates)?;
    workspace.package()?;
    let archives = workspace
        .crates
        .iter()
        .map(|(name, package)| {
            let path = workspace
                .target_directory
                .join("package")
                .join(format!("{name}-{}.crate", package.version));
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            Ok((name.clone(), Archive::read(&bytes, name, &package.version)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    validate(&workspace.crates, order, archives, registry)
}

fn validate(
    crates: &BTreeMap<String, Crate>,
    order: Vec<String>,
    archives: BTreeMap<String, Archive>,
    registry: &impl Registry,
) -> Result<ReleasePlan> {
    let mut pending = Vec::new();
    let mut already_published = Vec::new();
    let mut errors = Vec::new();
    let mut states = BTreeMap::new();
    let mut bumps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in &order {
        let package = &crates[name];
        eprintln!("Checking {} {} against crates.io...", name, package.version);
        match inspect(package, &archives[name], registry) {
            Ok(state) => {
                match &state {
                    Publication::Published => {
                        already_published.push(format!("{name} {}", package.version))
                    }
                    Publication::Unpublished => pending.push(package.clone()),
                    Publication::NeedsBump(reason) => {
                        bumps.insert(name.clone(), vec![reason.clone()]);
                    }
                }
                states.insert(name.clone(), state);
            }
            Err(error) => errors.push(format!("{name}: {error:#}")),
        }
    }

    // Propagate *future* edits, before any versions have been changed. Updating
    // a dependency's minimum changes a published consumer's manifest, requiring
    // another bump. A dependency-first traversal computes the full closure.
    for name in &order {
        let package = &crates[name];
        let dependencies: Vec<_> = package
            .dependencies
            .iter()
            .filter(|dependency| bumps.contains_key(*dependency))
            .cloned()
            .collect();
        if dependencies.is_empty() {
            continue;
        }
        let reason = format!(
            "update dependency requirements after bumping {}",
            dependencies.join(", ")
        );
        match states.get(name) {
            Some(Publication::Published | Publication::NeedsBump(_)) => {
                bumps.entry(name.clone()).or_default().push(reason);
            }
            Some(Publication::Unpublished) => {
                // This version is still available: edit its requirements, but
                // do not demand another version bump or propagate a fictitious one.
                errors.push(format!(
                    "{name} {}: {reason}; its unpublished package version can stay unchanged",
                    package.version
                ));
            }
            None => {} // Registry failures remain errors, not invented bump decisions.
        }
    }
    for name in &order {
        if let Some(reasons) = bumps.get(name) {
            errors.push(format!(
                "{name} {}: bump the package version; {}",
                crates[name].version,
                reasons.join("; ")
            ));
        }
    }
    ensure!(
        errors.is_empty(),
        "release validation failed:\n  {}",
        errors.join("\n  ")
    );
    Ok(ReleasePlan {
        pending,
        already_published,
        archives,
    })
}

/// Used both during planning and immediately before each real upload, so a
/// competing publisher or a previous partial run cannot silently overwrite work.
pub(crate) fn is_published(
    package: &Crate,
    local: &Archive,
    registry: &impl Registry,
) -> Result<bool> {
    match inspect(package, local, registry)? {
        Publication::Published => Ok(true),
        Publication::Unpublished => Ok(false),
        Publication::NeedsBump(reason) => bail!("{reason}; bump the package version"),
    }
}

enum Publication {
    Unpublished,
    Published,
    NeedsBump(String),
}

fn inspect(package: &Crate, local: &Archive, registry: &impl Registry) -> Result<Publication> {
    let versions = registry.versions(&package.name)?;
    let latest = versions
        .iter()
        .filter(|v| v.version.pre.is_empty() && v.version.build.is_empty())
        .max_by(|a, b| a.version.cmp(&b.version));
    if let Some(latest) = latest
        && package.version < latest.version
    {
        return Ok(Publication::NeedsBump(format!(
            "local version {} is behind published version {}; choose a version above {}",
            package.version, latest.version, latest.version
        )));
    }
    let Some(published) = versions.iter().find(|v| v.version == package.version) else {
        return Ok(Publication::Unpublished);
    };
    if published.yanked {
        return Ok(Publication::NeedsBump(format!(
            "{} is yanked; choose a new version",
            package.version
        )));
    }
    let remote = registry.archive(&package.name, published)?;
    let differences = local.differences(&remote);
    if differences.is_empty() {
        Ok(Publication::Published)
    } else {
        Ok(Publication::NeedsBump(format!(
            "{} is already published, but package contents differ ({})",
            package.version,
            differences
                .into_iter()
                .take(8)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn dependency_order(crates: &BTreeMap<String, Crate>) -> Result<Vec<String>> {
    let mut remaining: BTreeSet<_> = crates.keys().cloned().collect();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .find(|name| {
                crates[*name]
                    .dependencies
                    .iter()
                    .all(|dep| !remaining.contains(dep))
            })
            .cloned();
        let Some(next) = next else {
            bail!(
                "publish dependency cycle among: {}",
                remaining.into_iter().collect::<Vec<_>>().join(", ")
            );
        };
        for dependency in &crates[&next].dependencies {
            ensure!(
                crates.contains_key(dependency),
                "{next}: missing publishable dependency {dependency}"
            );
        }
        remaining.remove(&next);
        ordered.push(next);
    }
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::PublishedVersion;

    fn package(name: &str, version: &str, dependencies: &[&str]) -> Crate {
        Crate {
            name: name.into(),
            version: version.parse().unwrap(),
            manifest_path: "Cargo.toml".into(),
            dependencies: dependencies.iter().map(|s| (*s).into()).collect(),
        }
    }

    struct MockRegistry {
        versions: BTreeMap<String, Vec<PublishedVersion>>,
        archives: BTreeMap<String, Archive>,
    }

    impl Registry for MockRegistry {
        fn versions(&self, name: &str) -> Result<Vec<PublishedVersion>> {
            Ok(self.versions.get(name).cloned().unwrap_or_default())
        }
        fn archive(&self, name: &str, _: &PublishedVersion) -> Result<Archive> {
            Ok(self.archives[name].clone())
        }
    }

    fn registry(name: &str, version: &str, content: &str) -> MockRegistry {
        MockRegistry {
            versions: BTreeMap::from([(
                name.into(),
                vec![PublishedVersion {
                    version: version.parse().unwrap(),
                    checksum: String::new(),
                    yanked: false,
                }],
            )]),
            archives: BTreeMap::from([(name.into(), Archive::fixture(content))]),
        }
    }

    #[test]
    fn dependency_order_is_deterministic_and_detects_cycles() {
        let crates = [
            package("api", "1.0.0", &["core", "optional"]),
            package("core", "1.0.0", &[]),
            package("optional", "1.0.0", &["core"]),
        ]
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();
        assert_eq!(
            dependency_order(&crates).unwrap(),
            ["core", "optional", "api"]
        );
        let crates = [package("a", "1.0.0", &["b"]), package("b", "1.0.0", &["a"])]
            .into_iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        assert!(dependency_order(&crates).is_err());
    }

    #[test]
    fn unchanged_published_versions_are_safe_to_resume() {
        assert!(
            is_published(
                &package("demo", "1.0.0", &[]),
                &Archive::fixture("same"),
                &registry("demo", "1.0.0", "same")
            )
            .unwrap()
        );
    }

    #[test]
    fn missing_bump_and_downgrade_fail() {
        let registry = registry("demo", "1.0.0", "old");
        assert!(
            is_published(
                &package("demo", "1.0.0", &[]),
                &Archive::fixture("changed"),
                &registry
            )
            .unwrap_err()
            .to_string()
            .contains("bump the package version")
        );
        assert!(
            is_published(
                &package("demo", "0.9.0", &[]),
                &Archive::fixture("old"),
                &registry
            )
            .is_err()
        );
        assert!(
            !is_published(
                &package("demo", "1.0.1", &[]),
                &Archive::fixture("changed"),
                &registry
            )
            .unwrap()
        );
        assert!(
            !is_published(
                &package("new", "0.1.0", &[]),
                &Archive::fixture("new"),
                &registry
            )
            .unwrap()
        );
    }

    #[test]
    fn yanked_versions_cannot_be_reused() {
        let mut registry = registry("demo", "1.0.0", "same");
        registry.versions.get_mut("demo").unwrap()[0].yanked = true;
        assert!(
            is_published(
                &package("demo", "1.0.0", &[]),
                &Archive::fixture("same"),
                &registry
            )
            .is_err()
        );
    }

    #[test]
    fn plan_collects_all_missing_bumps_before_publishing() {
        let crates = [package("a", "1.0.0", &[]), package("b", "1.0.0", &["a"])]
            .into_iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        let b = registry("b", "1.0.0", "old");
        let mut registry = registry("a", "1.0.0", "old");
        registry.versions.extend(b.versions);
        registry.archives.extend(b.archives);
        let archives = BTreeMap::from([
            ("a".into(), Archive::fixture("new")),
            ("b".into(), Archive::fixture("new")),
        ]);
        let error = validate(
            &crates,
            dependency_order(&crates).unwrap(),
            archives,
            &registry,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("a 1.0.0:") && error.contains("b 1.0.0:"));
    }

    #[test]
    fn predicts_all_transitive_bumps_before_any_manifest_is_edited() {
        let packages = [
            package("lita-tokenizers-kytea", "0.1.1", &[]),
            package("lita-tokenizers-mecab", "0.1.0", &[]),
            package(
                "lita-tokenizers",
                "0.2.1",
                &["lita-tokenizers-kytea", "lita-tokenizers-mecab"],
            ),
            package("lita-tokenizers-cli", "0.1.1", &["lita-tokenizers"]),
            package("unrelated", "1.0.0", &[]),
        ];
        let mut snapshot = MockRegistry {
            versions: BTreeMap::new(),
            archives: BTreeMap::new(),
        };
        let mut archives = BTreeMap::new();
        for package in &packages {
            let published = registry(&package.name, &package.version.to_string(), "published");
            snapshot.versions.extend(published.versions);
            snapshot.archives.extend(published.archives);
            let changed = matches!(
                package.name.as_str(),
                "lita-tokenizers-kytea" | "lita-tokenizers-mecab" | "lita-tokenizers-cli"
            );
            archives.insert(
                package.name.clone(),
                Archive::fixture(if changed { "changed" } else { "published" }),
            );
        }
        let crates = packages.into_iter().map(|p| (p.name.clone(), p)).collect();
        let error = validate(
            &crates,
            dependency_order(&crates).unwrap(),
            archives,
            &snapshot,
        )
        .err()
        .unwrap()
        .to_string();
        for expected in [
            "lita-tokenizers-kytea 0.1.1",
            "lita-tokenizers-mecab 0.1.0",
            "lita-tokenizers 0.2.1",
            "lita-tokenizers-cli 0.1.1",
        ] {
            assert_eq!(
                error
                    .lines()
                    .filter(|line| line.trim_start().starts_with(&format!("{expected}:")))
                    .count(),
                1,
                "{error}"
            );
        }
        assert!(!error.contains("unrelated"), "{error}");
        assert!(error.contains("update dependency requirements after bumping lita-tokenizers-kytea, lita-tokenizers-mecab"), "{error}");
        let cli = error
            .lines()
            .find(|line| line.trim_start().starts_with("lita-tokenizers-cli 0.1.1:"))
            .unwrap();
        assert!(
            cli.contains("update dependency requirements after bumping lita-tokenizers"),
            "{error}"
        );
    }

    #[test]
    fn unpublished_consumer_needs_requirement_edits_but_not_another_bump() {
        let crates = [
            package("core", "1.0.0", &[]),
            package("api", "1.0.1", &["core"]),
        ]
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();
        let mut snapshot = registry("core", "1.0.0", "old");
        let api = registry("api", "1.0.0", "old");
        snapshot.versions.extend(api.versions);
        snapshot.archives.extend(api.archives);
        let archives = BTreeMap::from([
            ("core".into(), Archive::fixture("new")),
            ("api".into(), Archive::fixture("new")),
        ]);
        let error = validate(
            &crates,
            dependency_order(&crates).unwrap(),
            archives,
            &snapshot,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains("core 1.0.0: bump the package version"),
            "{error}"
        );
        assert!(error.contains("api 1.0.1: update dependency requirements after bumping core; its unpublished package version can stay unchanged"), "{error}");
        assert!(!error.contains("api 1.0.1: bump"), "{error}");
    }
}
