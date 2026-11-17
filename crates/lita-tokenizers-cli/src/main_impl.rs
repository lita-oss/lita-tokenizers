use tokenizers_api as api;

pub use anyhow::Error;
use anyhow::anyhow;
#[cfg(feature = "spacy")]
use clap::ValueEnum;
use clap::{Parser, Subcommand};

use std::ffi::OsStr;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    tokenizer: Tokenizer,
}

#[derive(Debug, Clone, Subcommand)]
enum Tokenizer {
    Kytea {
        input: Input,

        #[arg(short, long, default_value = "dynamic")]
        charset: Charset,

        #[arg(short, long)]
        model: PathBuf,

        #[arg(short, long)]
        tag: Option<Indices>,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Mecab {
        input: Input,

        #[arg(short, long, default_value = "dynamic")]
        charset: Charset,

        #[arg(short, long)]
        dict: PathBuf,

        #[arg(short, long)]
        rc: PathBuf,

        #[arg(short, long)]
        tag: Option<Indices>,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Spacy {
        input: Input,

        #[arg(short, long, default_value = "dynamic")]
        charset: Charset,

        #[arg(
            short,
            long,
            required_unless_present = "model_path",
            conflicts_with = "model_path"
        )]
        model: Option<String>,

        #[arg(long, required_unless_present = "model", conflicts_with = "model")]
        model_path: Option<PathBuf>,

        #[arg(long)]
        site_packages: Vec<PathBuf>,

        #[arg(short, long, default_value = "pos,tag,lemma,morph")]
        tag: SpacyAttributes,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Whitespace {
        input: Input,

        #[arg(short, long, default_value = "dynamic")]
        charset: Charset,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Input {
    Path(PathBuf),
    Stdin,
}

impl From<&OsStr> for Input {
    fn from(value: &OsStr) -> Self {
        if value == "-" {
            Self::Stdin
        } else {
            Self::Path(value.into())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Charset {
    Utf8,
    #[cfg(feature = "decode")]
    ShiftJis,
    #[cfg(feature = "decode")]
    EucJp,
    #[cfg(feature = "decode")]
    Dynamic,
}

impl From<&OsStr> for Charset {
    fn from(value: &OsStr) -> Self {
        #[cfg(feature = "decode")]
        fn from_impl(value: &OsStr) -> Charset {
            if value.eq_ignore_ascii_case("utf8")
                || value.eq_ignore_ascii_case("utf-8")
                || value.eq_ignore_ascii_case("utf_8")
            {
                Charset::Utf8
            } else if value.eq_ignore_ascii_case("shiftjis")
                || value.eq_ignore_ascii_case("shift-jis")
                || value.eq_ignore_ascii_case("shift_jis")
            {
                Charset::ShiftJis
            } else if value.eq_ignore_ascii_case("eucjp")
                || value.eq_ignore_ascii_case("euc-jp")
                || value.eq_ignore_ascii_case("euc_jp")
            {
                Charset::EucJp
            } else {
                Charset::Dynamic
            }
        }
        #[cfg(not(feature = "decode"))]
        fn from_impl(_value: &OsStr) -> Charset {
            Charset::Utf8
        }

        from_impl(value)
    }
}

#[derive(Debug, Clone)]
pub struct Indices {
    #[cfg(any(feature = "kytea", feature = "mecab"))]
    pub inner: Vec<usize>,
}

impl From<&OsStr> for Indices {
    fn from(value: &OsStr) -> Self {
        #[cfg(any(feature = "kytea", feature = "mecab"))]
        fn from_impl(value: &OsStr) -> Indices {
            let Some(values) = value.to_str() else {
                return Indices {
                    inner: Default::default(),
                };
            };

            let mut inner = Vec::new();
            for i in values.split(',') {
                let i = if let Ok(i) = i.parse() { i } else { usize::MAX };
                inner.push(i);
            }
            Indices { inner }
        }
        #[cfg(not(any(feature = "kytea", feature = "mecab")))]
        fn from_impl(_value: &OsStr) -> Indices {
            Indices {}
        }

        from_impl(value)
    }
}

#[cfg(feature = "spacy")]
#[derive(Debug, Clone, Copy, ValueEnum)]
enum SpacyAttribute {
    Pos,
    Tag,
    Lemma,
    Morph,
}

#[derive(Debug, Clone)]
struct SpacyAttributes {
    #[cfg(feature = "spacy")]
    inner: Vec<SpacyAttribute>,
}

impl FromStr for SpacyAttributes {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        #[cfg(feature = "spacy")]
        fn from_impl(value: &str) -> Result<SpacyAttributes, String> {
            let mut inner = Vec::new();
            if !value.is_empty() {
                for attribute in value.split(',') {
                    let attribute = <SpacyAttribute as ValueEnum>::from_str(attribute, false)?;
                    inner.push(attribute);
                }
            }
            Ok(SpacyAttributes { inner })
        }
        #[cfg(not(feature = "spacy"))]
        fn from_impl(_value: &str) -> Result<SpacyAttributes, String> {
            Ok(SpacyAttributes {})
        }

        from_impl(value)
    }
}

#[cfg(not(all(feature = "kytea", feature = "mecab", feature = "spacy")))]
fn tok_feature_disabled(tok: &'static str, feat: &'static str) -> Result<(), Error> {
    Err(anyhow!(
        "{tok} is disabled. To use it, enable the feature flag {feat}"
    ))
}

mod compat {
    use super as cli;
    use super::api;

    use api::Charset;

    use std::path::PathBuf;

    #[cfg(feature = "spacy")]
    pub(crate) struct Attributes<'a>(std::slice::Iter<'a, cli::SpacyAttribute>);

    #[cfg(feature = "spacy")]
    impl Iterator for Attributes<'_> {
        type Item = api::spacy::TokenAttribute;

        fn next(&mut self) -> Option<Self::Item> {
            use api::spacy::TokenAttribute;
            use cli::SpacyAttribute;

            self.0.next().map(|attribute| match attribute {
                SpacyAttribute::Pos => TokenAttribute::Pos,
                SpacyAttribute::Tag => TokenAttribute::Tag,
                SpacyAttribute::Lemma => TokenAttribute::Lemma,
                SpacyAttribute::Morph => TokenAttribute::Morph,
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }

    #[cfg(feature = "spacy")]
    impl api::spacy_core::TokenAttributes for cli::SpacyAttributes {
        type Iter<'a> = Attributes<'a>;

        fn attributes(&self) -> Self::Iter<'_> {
            Attributes(self.inner.iter())
        }
    }

    pub(crate) struct Input<'a>(pub(crate) &'a cli::Input);
    impl<'a> From<Input<'a>> for api::Input<'a> {
        fn from(value: Input<'a>) -> Self {
            use cli::Input;
            match value.0 {
                Input::Path(path) => Self::Path(path),
                Input::Stdin => Self::Stdin,
            }
        }
    }

    pub(crate) struct Output<'a>(pub(crate) &'a Option<PathBuf>);
    impl<'a> From<Output<'a>> for api::Output<'a> {
        fn from(value: Output<'a>) -> Self {
            match value.0 {
                Some(path) => Self::Path(path),
                None => Self::Stdout,
            }
        }
    }

    impl From<&cli::Charset> for Charset {
        fn from(value: &cli::Charset) -> Self {
            use cli::Charset;
            match value {
                Charset::Utf8 => Self::Utf8,
                #[cfg(feature = "decode")]
                Charset::ShiftJis => Self::ShiftJis,
                #[cfg(feature = "decode")]
                Charset::EucJp => Self::EucJp,
                #[cfg(feature = "decode")]
                Charset::Dynamic => Self::Dynamic,
            }
        }
    }

    #[cfg(any(feature = "kytea", feature = "mecab"))]
    pub(crate) struct Indices<'a>(pub(crate) &'a Option<cli::Indices>);

    #[cfg(feature = "kytea")]
    impl<'a> From<Indices<'a>> for api::kytea::TagIndex<'a> {
        fn from(value: Indices<'a>) -> Self {
            match value.0 {
                Some(v) => {
                    if v.inner.is_empty() {
                        Self::Asis
                    } else {
                        Self::Specified(&v.inner)
                    }
                }
                None => Self::Asis,
            }
        }
    }

    #[cfg(feature = "mecab")]
    impl<'a> From<Indices<'a>> for api::mecab::TagIndex<'a> {
        fn from(value: Indices<'a>) -> Self {
            match value.0 {
                Some(v) => {
                    if v.inner.is_empty() {
                        Self::Asis
                    } else {
                        Self::Specified(&v.inner)
                    }
                }
                None => Self::Asis,
            }
        }
    }
}

async fn run(args: Args) -> Result<(), Error> {
    match &args.tokenizer {
        #[cfg(feature = "kytea")]
        Tokenizer::Kytea {
            input,
            output,
            model,
            tag,
            charset,
        } => api::kytea::run(
            compat::Input(input).into(),
            compat::Output(output).into(),
            charset.into(),
            model,
            compat::Indices(tag).into(),
        )
        .await
        .map_err(|e| anyhow!("tokenization failed: {e}")),
        #[cfg(not(feature = "kytea"))]
        Tokenizer::Kytea { .. } => tok_feature_disabled("KyTea", "kytea"),
        #[cfg(feature = "mecab")]
        Tokenizer::Mecab {
            input,
            output,
            dict,
            rc,
            tag,
            charset,
        } => api::mecab::run(
            compat::Input(input).into(),
            compat::Output(output).into(),
            charset.into(),
            api::mecab::ModelPath { dict, rc },
            compat::Indices(tag).into(),
        )
        .await
        .map_err(|e| anyhow!("tokenization failed: {e}")),
        #[cfg(not(feature = "mecab"))]
        Tokenizer::Mecab { .. } => tok_feature_disabled("MeCab", "mecab"),
        #[cfg(feature = "spacy")]
        Tokenizer::Spacy {
            input,
            output,
            model,
            model_path,
            site_packages,
            tag,
            charset,
        } => {
            let model = match (model, model_path) {
                (Some(model), None) => api::spacy::Model::Package(model),
                (None, Some(path)) => api::spacy::Model::Path(path),
                _ => return Err(anyhow!("specify either --model or --model-path for spaCy")),
            };
            api::spacy::run(
                compat::Input(input).into(),
                compat::Output(output).into(),
                charset.into(),
                api::spacy_core::sys::Config::new(site_packages),
                model,
                tag,
            )
            .await
            .map_err(|e| anyhow!("tokenization failed: {e}"))
        }
        #[cfg(not(feature = "spacy"))]
        Tokenizer::Spacy { .. } => tok_feature_disabled("spaCy", "spacy"),
        Tokenizer::Whitespace {
            input,
            output,
            charset,
        } => api::whitespace::run(
            compat::Input(input).into(),
            compat::Output(output).into(),
            charset.into(),
        )
        .await
        .map_err(|e| anyhow!("tokenization failed: {e}")),
    }
}

pub async fn main_impl() -> Result<(), Error> {
    let args = Args::parse();

    run(args).await
}
