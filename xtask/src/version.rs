use anyhow::{Result, ensure};
use semver::{Op, Version, VersionReq};

pub(crate) fn parse(text: &str) -> Result<Version> {
    let version = Version::parse(text)?;
    ensure!(
        version.pre.is_empty() && version.build.is_empty(),
        "expected major.minor.patch, got {text}"
    );
    Ok(version)
}

/// Require a declared minimum at least as new as the version tested locally.
/// Checking only `matches` would let ^0.1.0 silently accept a locally used 0.1.2.
pub(crate) fn requires_current(requirement: &str, version: &Version) -> Result<bool> {
    let req = VersionReq::parse(requirement)?;
    if !req.matches(version) {
        return Ok(false);
    }
    Ok(req.comparators.iter().any(|cmp| {
        let (Some(minor), Some(patch)) = (cmp.minor, cmp.patch) else {
            return false;
        };
        let bound = Version::new(cmp.major, minor, patch);
        match cmp.op {
            Op::Exact | Op::GreaterEq | Op::Caret | Op::Tilde => bound >= *version,
            Op::Greater => {
                bound >= *version
                    || (bound.major == version.major
                        && bound.minor == version.minor
                        && bound.patch.checked_add(1) == Some(version.patch))
            }
            _ => false,
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_numeric_and_stable() {
        assert!(parse("0.9.9").unwrap() < parse("0.10.0").unwrap());
        for invalid in ["1.0", "1.0.0-alpha", "1.0.0+build", "-1.0.0"] {
            assert!(parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn checks_lower_bounds_not_just_compatibility() {
        let current = parse("0.1.2").unwrap();
        for req in ["0.1.2", "=0.1.2", "~0.1.2", ">=0.1.2, <0.2.0", ">0.1.1"] {
            assert!(requires_current(req, &current).unwrap(), "{req}");
        }
        for req in ["0.1.1", "*", "0.1", ">=0.1.0", "0.2.0", "<0.2.0", "=0.1.1"] {
            assert!(!requires_current(req, &current).unwrap(), "{req}");
        }
    }
}
