mod common;

use spacy_sys::python::types::PyStringMethods as _;
use spacy_sys::{Error, Python, TokenAttribute};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn token_views_preserve_unicode_offsets_and_annotations() -> Result<(), Error> {
    let model = common::environment().config().blank("en")?;
    assert_eq!(model.lang()?, "en");
    Python::attach(|py| {
        let doc = model.make_doc(py, "Café owners aren't naïve.")?;
        assert_eq!(doc.text()?, "Café owners aren't naïve.");
        assert_eq!(doc.len()?, 6);
        assert!(!doc.is_empty()?);
        let tokens = doc
            .tokens()?
            .map(|token| {
                let token = token?;
                Ok((
                    token.index()?,
                    token.idx()?,
                    token.text()?,
                    token.is_punct()?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        assert_eq!(
            tokens,
            [
                (0, 0, "Café".to_owned(), false),
                (1, 5, "owners".to_owned(), false),
                (2, 12, "are".to_owned(), false),
                (3, 15, "n't".to_owned(), false),
                (4, 19, "naïve".to_owned(), false),
                (5, 24, ".".to_owned(), true),
            ]
        );
        let token = doc.token(0)?.expect("first token");
        let text = token.text_ref()?;
        assert_eq!(text.to_str()?, "Café");
        assert!(token.is_alpha()?);
        assert!(!token.is_stop()?);
        assert!(!token.is_space()?);
        assert_eq!(token.is_sent_start()?, Some(true));
        assert_eq!(token.pos()?, "");
        assert_eq!(token.tag()?, "");
        assert_eq!(token.lemma()?, "");
        assert_eq!(token.morph()?, "");
        for attr in [
            TokenAttribute::Pos,
            TokenAttribute::Tag,
            TokenAttribute::Lemma,
            TokenAttribute::Morph,
        ] {
            assert_eq!(token.attribute(attr)?, "", "{attr:?}");
        }
        assert!(doc.token(6)?.is_none());
        assert!(doc.token(usize::MAX)?.is_none());
        Ok(())
    })
}

#[test]
fn sentence_spans_retain_space_tokens_and_order() -> Result<(), Error> {
    let model = common::environment().blank_english()?;
    Python::attach(|py| {
        let doc = model.process(py, "Hello. \nBye!")?;
        assert!(doc.has_annotation("SENT_START")?);
        let sentences = doc
            .sentences()?
            .map(|span| {
                let span = span?;
                let tokens = span
                    .tokens()?
                    .map(|token| token?.text())
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok((
                    span.text()?,
                    span.start()?,
                    span.end()?,
                    span.len()?,
                    span.is_empty()?,
                    tokens,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        assert_eq!(
            sentences,
            [
                (
                    "Hello.".to_owned(),
                    0,
                    2,
                    2,
                    false,
                    vec!["Hello".to_owned(), ".".to_owned()]
                ),
                (
                    "\nBye!".to_owned(),
                    2,
                    5,
                    3,
                    false,
                    vec!["\n".to_owned(), "Bye".to_owned(), "!".to_owned()]
                ),
            ]
        );
        assert_eq!(
            doc.token(1)?.expect("punctuation").is_sent_end()?,
            Some(true)
        );
        assert!(doc.token(2)?.expect("newline").is_space()?);
        Ok(())
    })
}

#[test]
fn documents_remain_valid_after_input_and_pipeline_are_dropped() -> Result<(), Error> {
    Python::attach(|py| {
        let doc = {
            let model = common::environment().blank_english()?;
            let input = String::from("Independent document.");
            model.process(py, &input)?
        };
        assert_eq!(doc.text()?, "Independent document.");
        let mut tokens = doc.tokens()?;
        assert_eq!(tokens.next().expect("first token")?.text()?, "Independent");
        assert_eq!(tokens.next().expect("second token")?.text()?, "document");
        assert_eq!(tokens.next().expect("punctuation")?.text()?, ".");
        assert!(tokens.next().is_none());
        assert!(tokens.next().is_none());
        Ok(())
    })
}

#[test]
fn empty_documents_and_iterators_are_terminal() -> Result<(), Error> {
    let model = common::environment().blank_english()?;
    Python::attach(|py| {
        let doc = model.process(py, "")?;
        assert!(doc.is_empty()?);
        assert_eq!(doc.len()?, 0);
        assert_eq!(doc.text()?, "");
        assert!(doc.token(0)?.is_none());
        let mut tokens = doc.tokens()?;
        assert!(tokens.next().is_none());
        assert!(tokens.next().is_none());
        let mut sentences = doc.sentences()?;
        assert!(sentences.next().is_none());
        assert!(sentences.next().is_none());
        Ok(())
    })
}

#[test]
fn sentence_iteration_returns_missing_annotation_error_once() -> Result<(), Error> {
    let model = common::environment().config().blank("en")?;
    Python::attach(|py| {
        let doc = model.process(py, "No sentence annotations.")?;
        let mut sentences = doc.sentences()?;
        let error = sentences
            .next()
            .expect("sentence generator should report an error")
            .err()
            .expect("sentence annotation is absent");
        assert!(error.to_string().contains("E030"), "{error}");
        assert!(sentences.next().is_none());
        assert!(sentences.next().is_none());
        Ok(())
    })
}

#[test]
fn pipeline_components_are_explicit_and_invalid_components_fail() -> Result<(), Error> {
    let mut model = common::environment().config().blank("en")?;
    assert!(model.pipe_names()?.is_empty());
    assert!(!model.has_pipe("sentencizer")?);
    model.add_pipe("sentencizer")?;
    assert!(model.has_pipe("sentencizer")?);
    assert_eq!(model.pipe_names()?, ["sentencizer"]);
    model.disable_pipe("sentencizer")?;
    assert!(!model.has_pipe("sentencizer")?);
    assert!(model.pipe_names()?.is_empty());
    model.enable_pipe("sentencizer")?;
    assert!(model.has_pipe("sentencizer")?);
    assert_eq!(model.pipe_names()?, ["sentencizer"]);
    model.remove_pipe("sentencizer")?;
    assert!(!model.has_pipe("sentencizer")?);
    let error = model
        .add_pipe("__spacy_sys_missing_component__")
        .expect_err("unknown component must fail");
    assert!(
        error
            .to_string()
            .contains("__spacy_sys_missing_component__"),
        "{error}"
    );
    Ok(())
}

#[test]
fn streaming_pipe_preserves_document_order_and_exhaustion() -> Result<(), Error> {
    let model = common::environment().blank_english()?;
    Python::attach(|py| {
        let mut docs = model.pipe(py, &["First.", "", "Café!"], 2)?;
        assert_eq!(docs.next().expect("first document")?.text()?, "First.");
        assert!(docs.next().expect("empty document")?.is_empty()?);
        assert_eq!(docs.next().expect("last document")?.text()?, "Café!");
        assert!(docs.next().is_none());
        assert!(docs.next().is_none());
        let mut docs = model.pipe(py, &[], 2)?;
        assert!(docs.next().is_none());
        Ok(())
    })
}

#[test]
fn trained_pipeline_supplies_pos_and_morphology() -> Result<(), Error> {
    let model = common::environment().config().load("en_core_web_sm")?;
    Python::attach(|py| {
        let doc = model.process(py, "We are programmers.")?;
        let token = doc.token(2)?.expect("programmers");
        assert_eq!(token.pos()?, "NOUN");
        assert_eq!(token.tag()?, "NNS");
        assert_eq!(token.lemma()?, "programmer");
        assert_eq!(token.morph()?, "Number=Plur");
        for (attr, expected) in [
            (TokenAttribute::Pos, "NOUN"),
            (TokenAttribute::Tag, "NNS"),
            (TokenAttribute::Lemma, "programmer"),
            (TokenAttribute::Morph, "Number=Plur"),
        ] {
            assert_eq!(token.attribute(attr)?, expected, "{attr:?}");
        }
        Ok(())
    })
}

#[cfg(feature = "entities")]
#[test]
fn named_entities_are_labeled_token_spans() -> Result<(), Error> {
    let model = common::environment().config().load("en_core_web_sm")?;
    Python::attach(|py| {
        let doc = model.process(py, "Alice works at Microsoft in London.")?;
        let entities = doc
            .entities()?
            .map(|span| {
                let span = span?;
                Ok((span.text()?, span.label()?, span.start()?, span.end()?))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        assert_eq!(
            entities,
            [
                ("Alice".to_owned(), "PERSON".to_owned(), 0, 1),
                ("Microsoft".to_owned(), "ORG".to_owned(), 3, 4),
                ("London".to_owned(), "GPE".to_owned(), 5, 6),
            ]
        );
        let token = doc.token(3)?.expect("Microsoft");
        assert_eq!(token.ent_iob()?, "B");
        assert_eq!(token.ent_type()?, "ORG");
        assert_eq!(token.attribute(TokenAttribute::EntIob)?, "B");
        assert_eq!(token.attribute(TokenAttribute::EntType)?, "ORG");
        Ok(())
    })
}

#[cfg(feature = "parsing")]
#[test]
fn dependencies_expose_heads_and_ordered_children() -> Result<(), Error> {
    let model = common::environment().config().load("en_core_web_sm")?;
    Python::attach(|py| {
        let doc = model.process(py, "Alice works at Microsoft in London.")?;
        let subject = doc.token(0)?.expect("subject");
        assert_eq!(subject.dep()?, "nsubj");
        assert_eq!(subject.attribute(TokenAttribute::Dep)?, "nsubj");
        assert_eq!(subject.head()?.text()?, "works");
        let verb = subject.head()?;
        assert_eq!(verb.dep()?, "ROOT");
        let children = verb
            .children()?
            .map(|token| token?.text())
            .collect::<Result<Vec<_>, Error>>()?;
        assert_eq!(children, ["Alice", "at", "."]);
        let mut children = subject.children()?;
        assert!(children.next().is_none());
        assert!(children.next().is_none());
        Ok(())
    })
}

#[cfg(feature = "serialization")]
#[test]
fn serialized_pipeline_loads_from_a_unicode_directory() -> Result<(), Error> {
    let environment = common::environment();
    let path = environment.root.join("serialized model 日本語");
    let model = environment.blank_english()?;
    model.to_disk(&path)?;
    let mut restored_model = environment.blank_english()?;
    restored_model.disable_pipe("sentencizer")?;
    restored_model.read_disk(&path)?;
    assert!(restored_model.pipe_names()?.is_empty());
    restored_model.enable_pipe("sentencizer")?;
    assert_eq!(restored_model.pipe_names()?, ["sentencizer"]);
    Python::attach(|py| -> Result<(), Error> {
        let doc = restored_model.process(py, "Restored. Pipeline!")?;
        assert_eq!(doc.sentences()?.count(), 2);
        Ok(())
    })?;
    let model = environment.config().load_path(&path)?;
    Python::attach(|py| {
        let doc = model.process(py, "Hello. Goodbye!")?;
        assert_eq!(doc.sentences()?.count(), 2);
        let bytes = doc.to_bytes()?;
        assert!(!bytes.is_empty());
        let mut restored = model.make_doc(py, "")?;
        restored.read_bytes(&bytes)?;
        assert_eq!(restored.text()?, doc.text()?);
        assert_eq!(restored.sentences()?.count(), 2);
        let path = environment.root.join("document.spacy");
        doc.to_disk(&path)?;
        let mut from_disk = model.make_doc(py, "")?;
        from_disk.read_disk(&path)?;
        assert_eq!(from_disk.text()?, "Hello. Goodbye!");
        assert_eq!(from_disk.sentences()?.count(), 2);
        Ok(())
    })
}

#[test]
fn import_sources_work_in_fresh_interpreters() -> Result<(), Error> {
    const SCENARIO: &str = "LITA_SPACY_IMPORT_TEST_SCENARIO";
    let environment = common::environment();
    let model_path = environment.root.join("model directory 日本語");
    let model_site = environment.root.join("custom-model-site");
    let pth_site = environment.root.join("pth-site");

    if let Ok(scenario) = std::env::var(SCENARIO) {
        // Each branch is run in a fresh process: sys.modules and paths added by another test
        // cannot hide a failure to configure the import path before the first spaCy import.
        let model = match scenario.as_str() {
            "searchable" => spacy_sys::Language::load("en_core_web_sm")?,
            "site-packages" => environment.config().load("en_core_web_sm")?,
            "owned-paths" => {
                let paths = environment
                    .site_packages()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>();
                spacy_sys::Config::new(paths).load("en_core_web_sm")?
            }
            "lazy-paths" => {
                let paths = environment.site_packages().map(PathBuf::from);
                let mut model = spacy_sys::Config::new(paths).blank("en")?;
                model.add_pipe("sentencizer")?;
                model
            }
            "model-pth" => {
                let paths = environment
                    .site_packages()
                    .chain(std::iter::once(pth_site.as_path()));
                spacy_sys::Config::new(paths).load("lita_spacy_test_model")?
            }
            "model-path" => {
                spacy_sys::Config::new(environment.site_packages()).load_path(&model_path)?
            }
            _ => panic!("unknown import-test scenario: {scenario}"),
        };
        Python::attach(|py| -> Result<(), Error> {
            let doc = model.process(py, "Hello world.")?;
            let tokens = doc
                .tokens()?
                .map(|token| token?.text())
                .collect::<Result<Vec<_>, Error>>()?;
            assert_eq!(tokens, ["Hello", "world", "."]);
            Ok(())
        })?;
        return Ok(());
    }

    environment.python(
        r#"
from pathlib import Path
import sys
import spacy

model_path, model_site, pth_site = map(Path, sys.argv[1:])
pipeline = spacy.blank("en")
pipeline.add_pipe("sentencizer")
pipeline.to_disk(model_path)

# A real local model package, including the distribution metadata spaCy uses to recognize
# installed models. Its package and pipeline live outside spaCy's own site-packages.
package = model_site / "lita_spacy_test_model"
package.mkdir(parents=True, exist_ok=True)
pipeline.to_disk(package / "pipeline")
(package / "__init__.py").write_text(
    "from pathlib import Path\nimport spacy\n"
    "def load(**kwargs):\n"
    "    return spacy.load(Path(__file__).parent / 'pipeline', **kwargs)\n"
)
metadata = model_site / "lita_spacy_test_model-1.0.0.dist-info"
metadata.mkdir(exist_ok=True)
(metadata / "METADATA").write_text(
    "Metadata-Version: 2.1\nName: lita-spacy-test-model\nVersion: 1.0.0\n"
)
pth_site.mkdir(exist_ok=True)
(pth_site / "models.pth").write_text(str(model_site) + "\n")
"#,
        &[&model_path, &model_site, &pth_site],
    );

    for scenario in [
        "searchable",
        "site-packages",
        "owned-paths",
        "lazy-paths",
        "model-pth",
        "model-path",
    ] {
        let mut command = Command::new(std::env::current_exe().expect("test executable path"));
        command
            .args([
                "--exact",
                "import_sources_work_in_fresh_interpreters",
                "--nocapture",
            ])
            .env(SCENARIO, scenario)
            .env_remove("PYTHONPATH");
        if scenario == "searchable" {
            command.env(
                "PYTHONPATH",
                std::env::join_paths(environment.site_packages()).expect("site-packages paths"),
            );
        }
        let output = command
            .output()
            .expect("could not run isolated import test");
        assert!(
            output.status.success(),
            "import scenario {scenario} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Ok(())
}

#[test]
fn missing_spacy_returns_the_import_error_in_a_fresh_interpreter() -> Result<(), Error> {
    const CHILD: &str = "SPACY_SYS_MISSING_IMPORT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        use spacy_sys::python::exceptions::PyModuleNotFoundError;
        use spacy_sys::python::prelude::*;

        Python::attach(|py| -> Result<(), Error> {
            // Initialize Python before removing its module search path. This tests the missing
            // spaCy package, rather than a failure to locate the Python standard library.
            py.import("sys")?.setattr("path", Vec::<String>::new())?;
            let error = spacy_sys::Language::blank("en")
                .err()
                .expect("spaCy must be unavailable with an empty search path");
            assert!(error.is_instance_of::<PyModuleNotFoundError>(py), "{error}");
            assert_eq!(
                error.value(py).getattr("name")?.extract::<String>()?,
                "spacy"
            );
            Ok(())
        })?;
        return Ok(());
    }

    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .args([
            "--exact",
            "missing_spacy_returns_the_import_error_in_a_fresh_interpreter",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .expect("could not run isolated missing-import test");
    assert!(
        output.status.success(),
        "missing-import test failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    Ok(())
}
