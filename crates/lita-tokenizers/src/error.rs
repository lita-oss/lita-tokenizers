use tokenizers_utils::UnknownEncoding;
use tokenizers_utils::fs::FsError;

use std::error::Error as StdError;
#[cfg(any(feature = "kytea", feature = "mecab"))]
use std::ffi::NulError;
use std::fmt;
use std::io::Error as IoError;

#[cfg(feature = "kytea")]
use tokenizers_kytea::Error as KyTeaError;

#[cfg(feature = "mecab")]
use tokenizers_mecab::sys::Error as MeCabError;

#[cfg(feature = "spacy")]
use tokenizers_spacy::Error as SpacyError;

#[derive(Debug)]
pub enum Error {
    /// Input text is of unknown encoding.
    UnknownEncoding(UnknownEncoding),
    /// IO error for reading inputs.
    ReadIo(IoError),
    /// IO error for writing outputs.
    WriteIo(IoError),
    /// Filesystem errors.
    Fs(FsError),

    #[cfg(feature = "kytea")]
    /// KyTea tokenization fails.
    KyTea(KyTeaError),
    #[cfg(feature = "kytea")]
    /// Model path is invalid as [C string](std::ffi::CString::new()).
    KyTeaModelPath(NulError),
    /// Could not read the KyTea model.
    #[cfg(feature = "kytea")]
    KyTeaModelRead(IoError),

    #[cfg(feature = "mecab")]
    /// MeCab tokenization fails.
    MeCab(MeCabError),
    #[cfg(feature = "mecab")]
    /// Model or dictrc path is invalid as [C string](std::ffi::CString::new()).
    MeCabModelPath(NulError),
    #[cfg(feature = "mecab")]
    /// Failed to initialize the MeCab model.
    MeCabModelRead(MeCabError),

    #[cfg(feature = "spacy")]
    /// spaCy tokenization fails.
    Spacy(SpacyError),
    #[cfg(feature = "spacy")]
    /// Failed to import spaCy or load the pipeline.
    SpacyModelRead(SpacyError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEncoding(e) => write!(f, "invalid encoding for the input: {e}"),
            Self::ReadIo(e) => write!(f, "could not read the input: {e}"),
            Self::WriteIo(e) => write!(f, "could not write to the output: {e}"),
            Self::Fs(e) => e.fmt(f),
            #[cfg(feature = "kytea")]
            Self::KyTea(e) => write!(f, "KyTea failed: {e}"),
            #[cfg(feature = "kytea")]
            Self::KyTeaModelPath(e) => write!(f, "KyTea model path is an invalid string: {e}"),
            #[cfg(feature = "kytea")]
            Self::KyTeaModelRead(e) => write!(f, "could not read the KyTea model: {e}"),

            #[cfg(feature = "mecab")]
            Self::MeCab(e) => write!(f, "MeCab failed: {e}"),
            #[cfg(feature = "mecab")]
            Self::MeCabModelPath(e) => write!(f, "MeCab model path is an invalid string: {e}"),
            #[cfg(feature = "mecab")]
            Self::MeCabModelRead(e) => write!(f, "could not read the MeCab model: {e}"),

            #[cfg(feature = "spacy")]
            Self::Spacy(e) => write!(f, "spaCy failed: {e}"),
            #[cfg(feature = "spacy")]
            Self::SpacyModelRead(e) => write!(f, "could not load the spaCy model: {e}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::UnknownEncoding(e) => Some(e),
            Self::ReadIo(e) => Some(e),
            Self::WriteIo(e) => Some(e),
            Self::Fs(e) => Some(e),
            #[cfg(feature = "kytea")]
            Self::KyTea(e) => Some(e),
            #[cfg(feature = "kytea")]
            Self::KyTeaModelPath(e) => Some(e),
            #[cfg(feature = "kytea")]
            Self::KyTeaModelRead(e) => Some(e),
            #[cfg(feature = "mecab")]
            Self::MeCab(e) => Some(e),
            #[cfg(feature = "mecab")]
            Self::MeCabModelPath(e) => Some(e),
            #[cfg(feature = "mecab")]
            Self::MeCabModelRead(e) => Some(e),
            #[cfg(feature = "spacy")]
            Self::Spacy(e) => Some(e),
            #[cfg(feature = "spacy")]
            Self::SpacyModelRead(e) => Some(e),
        }
    }
}
