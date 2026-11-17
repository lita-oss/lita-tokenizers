//! spaCy tokenizer APIs.

use super::{Error, Input, Output, common};
use common::TokenizerError;

use tokenizers_core::Tokenizer;

use tokenizers_utils::Charset;

pub use tokenizers_spacy::TokenAttribute;
use tokenizers_spacy::sys::Config;
use tokenizers_spacy::{Spacy, SpacyStreamBuilder, TokenAttributes};

use std::path::Path;

/// Represents where the model files are.
pub enum Model<'a> {
    /// An installed pipeline package or a UTF-8 model directory accepted by spaCy's `load()`.
    Package(&'a str),
    /// A model directory, preserving non-UTF-8 OS paths.
    Path(&'a Path),
}

impl<A: TokenAttributes> From<TokenizerError<SpacyStreamBuilder<'_, A>>> for Error {
    fn from(value: TokenizerError<SpacyStreamBuilder<'_, A>>) -> Self {
        Self::Spacy(value.0)
    }
}

/// Initializes the [spaCy model](tokenizers_spacy::sys::Language) and reads the model.
pub fn tokenizer<I, P, A>(
    config: Config<I>,
    model: Model<'_>,
    attributes: A,
) -> Result<Spacy<A>, Error>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let model = match model {
        Model::Package(name) => config.load(name),
        Model::Path(path) => config.load_path(path),
    }
    .map_err(Error::SpacyModelRead)?;

    Ok(Spacy::new(model, attributes))
}

/// Executes [`tokenize()`](common::tokio::tokenize()) with spaCy initialized by [`tokenizer()`].
#[cfg(feature = "tokio")]
pub async fn run<I, P, A>(
    input: Input<'_>,
    output: Output<'_>,
    charset: Charset,
    config: Config<I>,
    model: Model<'_>,
    attributes: A,
) -> Result<(), Error>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    A: TokenAttributes,
{
    let mut tok = tokenizer(config, model, attributes)?;
    let mut tok = tok.builder().map_err(Error::Spacy)?;

    common::tokio::tokenize(&mut tok, input, output, charset).await?;

    Ok(())
}

/// Executes [`tokenize()`](common::sync::tokenize()) with spaCy initialized by [`tokenizer()`].
pub fn run_sync<I, P, A>(
    input: Input<'_>,
    output: Output<'_>,
    charset: Charset,
    config: Config<I>,
    model: Model<'_>,
    attributes: A,
) -> Result<(), Error>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    A: TokenAttributes,
{
    let mut tok = tokenizer(config, model, attributes)?;
    let mut tok = tok.builder().map_err(Error::Spacy)?;

    common::sync::tokenize(&mut tok, input, output, charset)?;

    Ok(())
}
