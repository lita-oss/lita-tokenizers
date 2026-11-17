use super::*;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

fn site_packages() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    fs::create_dir_all(&root).unwrap();
    let packages = root.join("site-packages");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("site-packages.lock"))
        .unwrap();

    lock.lock_shared().unwrap();
    if packages.join("spacy").is_dir() {
        return packages;
    }

    // Upgrading a held shared lock can deadlock.
    lock.unlock().unwrap();
    lock.lock().unwrap();
    // Another process may have installed spaCy while we waited.
    if !packages.join("spacy").is_dir() {
        let python = std::env::var_os("PYO3_PYTHON").expect("PYO3_PYTHON must be set");
        let status = Command::new("uv")
            .args(["pip", "install", "--no-python-downloads", "--python"])
            .arg(python)
            .arg("--target")
            .arg(&packages)
            .arg("spacy==3.8.18")
            .status()
            .expect("could not run uv to install spaCy");
        if !status.success() {
            if packages.exists() {
                fs::remove_dir_all(&packages).unwrap();
            }
            panic!("could not install spaCy: {status}");
        }
        assert!(packages.join("spacy").is_dir(), "spaCy was not installed");
    }

    packages
}

fn model_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/models/lita-en-minimal")
        .canonicalize()
        .unwrap()
}

#[test]
fn tokenize() {
    let model = Config::new([site_packages()])
        .load_path(model_path())
        .unwrap();

    Python::attach(|py| {
        let doc = model
            .process(py, "The quick brown fox jumps over the lazy dog.")
            .unwrap();
        assert_eq!(
            doc.text().unwrap(),
            "The quick brown fox jumps over the lazy dog."
        );

        let expected = [
            ("The", "DET"),
            ("quick", "ADJ"),
            ("brown", "ADJ"),
            ("fox", "NOUN"),
            ("jumps", "VERB"),
            ("over", "ADP"),
            ("the", "DET"),
            ("lazy", "ADJ"),
            ("dog", "NOUN"),
            (".", "PUNCT"),
        ]
        .map(|(text, pos)| (text.to_owned(), pos.to_owned()));
        let found = doc
            .tokens()
            .unwrap()
            .map(|token| {
                let token = token.unwrap();
                (token.text().unwrap(), token.pos().unwrap())
            })
            .collect::<Vec<_>>();

        assert_eq!(found, expected);

        #[cfg(feature = "parsing")]
        {
            let expected = [
                (3, "det"),
                (3, "amod"),
                (3, "amod"),
                (4, "nsubj"),
                (4, "ROOT"),
                (8, "case"),
                (8, "det"),
                (8, "amod"),
                (4, "obl"),
                (4, "punct"),
            ]
            .map(|(head, dep)| (head, dep.to_owned()));
            let found = doc
                .tokens()
                .unwrap()
                .map(|token| {
                    let token = token.unwrap();
                    (token.head().unwrap().index().unwrap(), token.dep().unwrap())
                })
                .collect::<Vec<_>>();

            assert_eq!(found, expected);
        }
    });
}
