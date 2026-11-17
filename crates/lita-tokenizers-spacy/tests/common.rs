use lita_tokenizers_spacy::sys::{Config, Error, Language};

use std::path::PathBuf;

pub struct TestEnvironment {
    site_packages: Vec<PathBuf>,
}

impl TestEnvironment {
    pub fn config(&self) -> Config<'_, PathBuf> {
        Config::new(&self.site_packages)
    }

    pub fn blank_english(&self) -> Result<Language, Error> {
        let mut model = self.config().blank("en")?;
        model.add_pipe("sentencizer")?;
        Ok(model)
    }
}

pub fn environment() -> TestEnvironment {
    let paths = std::env::var_os("SPACY_TEST_SITE_PACKAGES").expect(
        "SPACY_TEST_SITE_PACKAGES is missing; prepare dependencies with crates/spacy-sys/tests/run.sh",
    );
    TestEnvironment {
        site_packages: std::env::split_paths(&paths).collect(),
    }
}
