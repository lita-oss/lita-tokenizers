use std::path::{Path, PathBuf};
use std::process::Command;

use spacy_sys::{Config, Language};

pub struct TestEnvironment {
    pub root: PathBuf,
    pub python: PathBuf,
    site_packages: Vec<PathBuf>,
}

impl TestEnvironment {
    pub fn config(&self) -> Config<'_, PathBuf> {
        Config::new(&self.site_packages)
    }

    pub fn blank_english(&self) -> Result<Language, spacy_sys::Error> {
        let mut model = self.config().blank("en")?;
        model.add_pipe("sentencizer")?;
        Ok(model)
    }

    pub fn python(&self, script: &str, args: &[&Path]) {
        let output = Command::new(&self.python)
            .arg("-c")
            .arg(script)
            .args(args)
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .output()
            .expect("could not run the test fixture Python");
        assert!(
            output.status.success(),
            "fixture Python failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    pub fn site_packages(&self) -> impl Iterator<Item = &Path> {
        self.site_packages.iter().map(PathBuf::as_path)
    }
}

pub fn environment() -> TestEnvironment {
    fn required(name: &str) -> std::ffi::OsString {
        std::env::var_os(name).unwrap_or_else(|| {
            panic!("{name} is missing; prepare dependencies with crates/spacy-sys/tests/run.sh")
        })
    }
    let root = PathBuf::from(required("SPACY_TEST_ROOT")).join(env!("CARGO_PKG_NAME"));
    std::fs::create_dir_all(&root).expect("could not create test fixture directory");
    TestEnvironment {
        root,
        python: PathBuf::from(required("SPACY_TEST_PYTHON")),
        site_packages: std::env::split_paths(&required("SPACY_TEST_SITE_PACKAGES")).collect(),
    }
}
