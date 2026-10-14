//! `kytea-sys` is the FFI wrapper for [KyTea](http://www.phontron.com/kytea/).
//!
//! While [`ffi`] provides the FFI bindings, the top-level items are the safe wrapper for KyTea.
//!
//! For now, only the prediction mode is supported.
//!
//! # Examples
//!
//! Basic usage for Japanese morphological analysis:
//!
//! ```
//! # fn analyze() {
//! use kytea_sys::{CorpusFormat, KyTea, ModelFormat, StringStream};
//!
//! let model_path = c"/path/to/your/kytea/model";
//! let mut model = KyTea::new();
//!
//! model.read_model(model_path, ModelFormat::Unknown).unwrap();
//!
//! model.config().set_training(false).set_input_format(CorpusFormat::Raw);
//!
//! let mut input = {
//!     let mut ss = StringStream::new();
//!     ss.push("すもももももももものうち\n");
//!     ss
//! };
//! let mut output = StringStream::new();
//!
//! let mut ctx = model.context(&mut input, &mut output).unwrap();
//! while ctx.predict().unwrap().is_continue() {}
//!
//! println!("{}", String::from_utf8_lossy(output.as_bytes()));
//! # }
//! ```
//!
//! The above example uses [`StringStream`], which is a wrapper for C++ `std::stringstream`.
//! Instead, you can use [`Fstream`], which wraps C++ `std::fstream`, for input/output as well:
//!
//! ```
//! # fn analyze() {
//! use kytea_sys::{CorpusFormat, KyTea, ModelFormat, Fstream};
//!
//! let model_path = c"/path/to/your/kytea/model";
//! let mut model = KyTea::new();
//!
//! model.read_model(model_path, ModelFormat::Unknown).unwrap();
//!
//! model.config().set_training(false).set_input_format(CorpusFormat::Raw);
//!
//! let mut input = Fstream::open(c"/path/to/input").unwrap();
//! let mut output = Fstream::create(c"/path/to/output").unwrap();
//!
//! let mut ctx = model.context(&mut input, &mut output).unwrap();
//! while ctx.predict().unwrap().is_continue() {}
//! output.flush().unwrap();
//!
//! println!("{}", std::fs::read_to_string("/path/to/output").unwrap());
//! # }
//! ```

/// The FFI layer for KyTea.
pub mod ffi;

#[cfg(test)]
mod tests;

use std::ffi::CStr;
use std::ffi::c_void;
use std::fmt;
use std::io::Error as IoError;
use std::marker::PhantomData;
use std::ops::ControlFlow;

/// Returns the KyTea library version defined in `config.h`.
pub fn version() -> &'static str {
    // SAFETY: kytea_version returns a NUL-terminated string literal with static lifetime.
    unsafe { CStr::from_ptr(ffi::kytea_version()) }
        .to_str()
        .expect("KyTea version must be valid UTF-8")
}

/// The main entrypoint for KyTea. It wraps C++ `kytea::Kytea`.
///
/// # Examples
///
/// With [`StringStream`]:
///
/// ```
/// # fn analyze() {
/// use kytea_sys::{CorpusFormat, KyTea, ModelFormat, StringStream};
///
/// let model_path = c"/path/to/your/kytea/model";
/// let mut model = KyTea::new();
///
/// model.read_model(model_path, ModelFormat::Unknown).unwrap();
///
/// model.config().set_training(false).set_input_format(CorpusFormat::Raw);
///
/// let mut input = {
///     let mut ss = StringStream::new();
///     ss.push("すもももももももものうち\n");
///     ss
/// };
/// let mut output = StringStream::new();
///
/// let mut ctx = model.context(&mut input, &mut output).unwrap();
/// while ctx.predict().unwrap().is_continue() {}
///
/// println!("{}", String::from_utf8_lossy(output.as_bytes()));
/// # }
/// ```
///
/// With [`Fstream`]:
///
/// ```
/// # fn analyze() {
/// use kytea_sys::{CorpusFormat, KyTea, ModelFormat, Fstream};
///
/// let model_path = c"/path/to/your/kytea/model";
/// let mut model = KyTea::new();
///
/// model.read_model(model_path, ModelFormat::Unknown).unwrap();
///
/// model.config().set_training(false).set_input_format(CorpusFormat::Raw);
///
/// let mut input = Fstream::open(c"/path/to/input").unwrap();
/// let mut output = Fstream::create(c"/path/to/output").unwrap();
///
/// let mut ctx = model.context(&mut input, &mut output).unwrap();
/// while ctx.predict().unwrap().is_continue() {}
/// output.flush().unwrap();
///
/// println!("{}", std::fs::read_to_string("/path/to/output").unwrap());
/// # }
/// ```
pub struct KyTea {
    inner: *mut c_void,
}

impl Default for KyTea {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when setting up a KyTea [`Context`],
/// returned by [`KyTea::context()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainConfigError {
    /// No action is possible with the current configuration.
    DoNothing,
    /// Raw input format was selected but word segmentation is disabled.
    RawWithoutWordSegmentation,
    /// Word segmentation is requested but no model is loaded.
    NoWordSegmentationModel,
}

impl fmt::Display for TrainConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DoNothing => write!(f, "no action is possible with the current configuration"),
            Self::RawWithoutWordSegmentation => write!(
                f,
                "raw input format was selected but word segmentation is disabled"
            ),
            Self::NoWordSegmentationModel => {
                write!(f, "word segmentation is requested but no model is loaded")
            }
        }
    }
}
impl std::error::Error for TrainConfigError {}

impl KyTea {
    /// Creates a new KyTea model instance.
    pub fn new() -> Self {
        unsafe {
            let inner = ffi::kytea_model_new();
            debug_assert!(!inner.is_null());
            Self { inner }
        }
    }

    /// Reads a KyTea model from the specified file path.
    pub fn read_model(&mut self, model: &CStr, format: ModelFormat) -> Result<(), IoError> {
        let format = match format {
            ModelFormat::Binary => ffi::ModelFormat::Binary,
            ModelFormat::Text => ffi::ModelFormat::Text,
            ModelFormat::Unknown => ffi::ModelFormat::Unknown,
        };
        unsafe {
            let err = ffi::kytea_model_read_path(self.inner, model.as_ptr(), format);
            err.to_io()
        }
    }

    /// Reads a KyTea model from the specified file path.
    pub fn read_model_from_stream<Mod: Stream>(
        &mut self,
        model: &mut Mod,
        format: ModelFormat,
    ) -> Result<(), IoError> {
        let format = match format {
            ModelFormat::Binary => ffi::ModelFormat::Binary,
            ModelFormat::Text => ffi::ModelFormat::Text,
            ModelFormat::Unknown => ffi::ModelFormat::Unknown,
        };
        unsafe {
            let err = ffi::kytea_model_read_stream(self.inner, model.as_stream().inner, format);
            err.to_io()
        }
    }

    /// Returns the configuration object for this KyTea instance.
    pub fn config(&mut self) -> KyTeaConfig<'_> {
        unsafe {
            let inner = ffi::kytea_model_config(self.inner);
            debug_assert!(!inner.is_null());
            KyTeaConfig {
                inner,
                _marker: PhantomData,
            }
        }
    }

    /// Creates a context for prediction using the provided input and output streams.
    ///
    /// This method performs sanity checks on the current configuration before creating the context.
    ///
    /// ```
    /// # use kytea_sys::{KyTea, StringStream};
    /// # fn analyze(model: &mut KyTea) {
    /// let mut input = {
    ///     let mut ss = StringStream::new();
    ///     ss.push("すもももももももものうち\n");
    ///     ss
    /// };
    /// let mut output = StringStream::new();
    ///
    /// let mut ctx = model.context(&mut input, &mut output).unwrap();
    /// while ctx.predict().unwrap().is_continue() {}
    ///
    /// println!("{}", String::from_utf8_lossy(output.as_bytes()));
    /// # }
    /// ```
    pub fn context<'k, In: Stream, Out: Stream>(
        &'k mut self,
        input: &mut In,
        output: &mut Out,
    ) -> Result<Context<'k, In, Out>, TrainConfigError> {
        unsafe {
            let err = ffi::kytea_model_sanity_train(self.inner);
            match err {
                1 => return Err(TrainConfigError::DoNothing),
                2 => return Err(TrainConfigError::RawWithoutWordSegmentation),
                3 => return Err(TrainConfigError::NoWordSegmentationModel),
                _ => {}
            }

            let input = input.as_stream();
            let corpus_in = ffi::kytea_model_corpus(self.inner, input.inner, false);

            let output = output.as_stream();
            let corpus_out = ffi::kytea_model_corpus(self.inner, output.inner, true);

            ffi::kytea_model_prepare_train(self.inner, corpus_out);

            Ok(Context {
                input: CorpusIo {
                    inner: corpus_in,
                    _marker: PhantomData,
                },
                output: CorpusIo {
                    inner: corpus_out,
                    _marker: PhantomData,
                },
                kytea: self,
            })
        }
    }
}

impl Drop for KyTea {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                ffi::kytea_model_delete(self.inner);
            }
        }
    }
}

/// Configuration options for a KyTea model. It wraps C++ `kytea::KyteaConfig`.
pub struct KyTeaConfig<'a> {
    inner: *mut c_void,
    _marker: PhantomData<&'a mut KyTea>,
}

impl KyTeaConfig<'_> {
    /// Sets whether to output debug information.
    /// Defaults to [`DebugLevel::Silent`].
    pub fn set_debug(&mut self, debug_level: DebugLevel) -> &mut Self {
        use std::ffi::c_uint;
        let level: c_uint = match debug_level {
            DebugLevel::Silent => 0,
            DebugLevel::Simple => 1,
            DebugLevel::Detailed => 2,
            DebugLevel::Full => 3,
        };

        unsafe {
            ffi::kytea_config_set_debug(self.inner, level);
        }
        self
    }

    /// Sets whether the model is in training mode or in prediction mode.
    /// Defaults to `true`.
    pub fn set_training(&mut self, is_train: bool) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_training(self.inner, is_train);
        }
        self
    }

    /// Sets the format of the input corpus.
    /// Defaults to [`CorpusFormat::Raw`].
    pub fn set_input_format(&mut self, fmt: CorpusFormat) -> &mut Self {
        unsafe {
            let fmt = match fmt {
                CorpusFormat::Raw => ffi::CorpusFormat::Raw,
                CorpusFormat::Full => ffi::CorpusFormat::Full,
                CorpusFormat::Part => ffi::CorpusFormat::Part,
                CorpusFormat::Prob => ffi::CorpusFormat::Prob,
                CorpusFormat::Tok => ffi::CorpusFormat::Tok,
                CorpusFormat::Eda => ffi::CorpusFormat::Eda,
                CorpusFormat::Tags => ffi::CorpusFormat::Tags,
            };

            ffi::kytea_config_set_input_format(self.inner, fmt);
        }
        if matches!(fmt, CorpusFormat::Full | CorpusFormat::Tok) {
            unsafe {
                ffi::kytea_config_set_do_ws(self.inner, false);
            }
        }
        self
    }

    /// Sets the character used to separate words in the output.
    /// Defaults to [`ffi::defaults::WORD_BOUND`].
    pub fn set_word_bound(&mut self, word_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_word_bound(self.inner, word_bound.as_ptr());
        }
        self
    }
    /// Sets the character used to separate tags in words.
    /// Defaults to [`ffi::defaults::TAG_BOUND`].
    pub fn set_tag_bound(&mut self, tag_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_tag_bound(self.inner, tag_bound.as_ptr());
        }
        self
    }
    /// Sets the character used to separate candidates in full/partial annotation.
    /// Defaults to [`ffi::defaults::ELEM_BOUND`].
    pub fn set_elem_bound(&mut self, elem_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_elem_bound(self.inner, elem_bound.as_ptr());
        }
        self
    }
    /// Sets the character used for unknown word boundaries.
    /// Defaults to [`ffi::defaults::UNK_BOUND`].
    pub fn set_unk_bound(&mut self, unk_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_unk_bound(self.inner, unk_bound.as_ptr());
        }
        self
    }
    /// Sets the character representing no word boundary.
    /// Defaults to [`ffi::defaults::NO_BOUND`].
    pub fn set_no_bound(&mut self, no_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_no_bound(self.inner, no_bound.as_ptr());
        }
        self
    }
    /// Sets the character representing a word boundary.
    /// Defaults to [`ffi::defaults::HAS_BOUND`].
    pub fn set_has_bound(&mut self, has_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_has_bound(self.inner, has_bound.as_ptr());
        }
        self
    }
    /// Sets the character representing a skipped word boundary.
    /// Defaults to [`ffi::defaults::SKIP_BOUND`].
    pub fn set_skip_bound(&mut self, skip_bound: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_skip_bound(self.inner, skip_bound.as_ptr());
        }
        self
    }
    /// Sets the escape character.
    /// Defaults to [`ffi::defaults::ESCAPE`].
    pub fn set_escape(&mut self, escape: &'static CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_escape(self.inner, escape.as_ptr());
        }
        self
    }

    /// Same as [`Self::set_word_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `word_bound`, so `word_bound` must outlive `self.`
    pub unsafe fn set_word_bound_unchecked(&mut self, word_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_word_bound(self.inner, word_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_tag_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `tag_bound`, so `tag_bound` must outlive `self.`
    pub unsafe fn set_tag_bound_unchecked(&mut self, tag_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_tag_bound(self.inner, tag_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_elem_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `elem_bound`, so `elem_bound` must outlive `self.`
    pub unsafe fn set_elem_bound_unchecked(&mut self, elem_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_elem_bound(self.inner, elem_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_unk_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `unk_bound`, so `unk_bound` must outlive `self.`
    pub unsafe fn set_unk_bound_unchecked(&mut self, unk_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_unk_bound(self.inner, unk_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_no_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `no_bound`, so `no_bound` must outlive `self.`
    pub unsafe fn set_no_bound_unchecked(&mut self, no_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_no_bound(self.inner, no_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_has_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `has_bound`, so `has_bound` must outlive `self.`
    pub unsafe fn set_has_bound_unchecked(&mut self, has_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_has_bound(self.inner, has_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_skip_bound()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `skip_bound`, so `skip_bound` must outlive `self.`
    pub unsafe fn set_skip_bound_unchecked(&mut self, skip_bound: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_skip_bound(self.inner, skip_bound.as_ptr());
        }
        self
    }
    /// Same as [`Self::set_escape()`].
    ///
    /// # Safety
    ///
    /// The inner model borrows `escape`, so `escape` must outlive `self.`
    pub unsafe fn set_escape_unchecked(&mut self, escape: &CStr) -> &mut Self {
        unsafe {
            ffi::kytea_config_set_escape(self.inner, escape.as_ptr());
        }
        self
    }
}

/// The debug level for the C++ half.
///
/// Defaults to `Silent`.
///
/// ```
/// # fn analyze() {
/// use kytea_sys::{KyTea, DebugLevel};
///
/// let mut model = KyTea::new();
/// model.config().set_debug(DebugLevel::Full);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DebugLevel {
    Silent,
    Simple,
    Detailed,
    Full,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelFormat {
    /// Binary file.
    Binary,
    /// Text file.
    Text,
    /// Either `Binary` or `Text`.
    #[default]
    Unknown,
}

/// Supported corpus formats for KyTea.
///
/// Defaults to `Raw` for input and `Full` for output.
///
/// ```
/// # fn analyze() {
/// use kytea_sys::{KyTea, CorpusFormat};
///
/// let mut model = KyTea::new();
/// model.config().set_input_format(CorpusFormat::Part);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CorpusFormat {
    /// Raw text.
    Raw,
    /// Fully annotated text.
    Full,
    /// Partially annotated text.
    Part,
    /// Text with tag probabilities.
    Prob,
    /// Tokenized text (word separation only).
    Tok,
    /// EDA format.
    Eda,
    /// Tagged format.
    Tags,
}

/// A handle to a KyTea IO stream.
/// It wraps C++ `std::iostream` class.
pub struct IoStream<'a> {
    inner: *mut c_void,
    _marker: PhantomData<&'a ()>,
}

/// A trait for types that can be treated as a KyTea IO stream.
pub trait Stream {
    /// Returns a handle to the IO stream.
    fn as_stream(&mut self) -> IoStream<'_>;
}

impl<S: Stream + ?Sized> Stream for &mut S {
    fn as_stream(&mut self) -> IoStream<'_> {
        S::as_stream(self)
    }
}

/// An in-memory IO stream for KyTea.
/// It wraps C++ `std::stringstream`.
///
/// ```
/// use kytea_sys::StringStream;
///
/// let mut ss = StringStream::new();
/// ss.push("foo bar");
/// assert_eq!(ss.as_bytes(), b"foo bar");
/// ```
pub struct StringStream {
    inner: *mut c_void,
}

impl Default for StringStream {
    fn default() -> Self {
        Self::new()
    }
}

impl StringStream {
    /// Creates a new empty [`StringStream`].
    pub fn new() -> Self {
        unsafe {
            let inner = ffi::kytea_stringstream_new();
            debug_assert!(!inner.is_null());
            Self { inner }
        }
    }

    /// Returns the contents of the stream as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let bytes = ffi::kytea_stringstream_as_slice(self.inner);
            debug_assert!(!bytes.ptr.is_null());
            std::slice::from_raw_parts(bytes.ptr as *const u8, bytes.size)
        }
    }

    /// Pushes a string into the stream.
    pub fn push(&mut self, input: &str) {
        unsafe {
            let input = ffi::Str {
                ptr: input.as_ptr() as *const _,
                size: input.len(),
            };
            ffi::kytea_stringstream_write(self.inner, input);
        }
    }
}

impl Drop for StringStream {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                ffi::kytea_stringstream_delete(self.inner);
            }
        }
    }
}

impl Stream for StringStream {
    fn as_stream(&mut self) -> IoStream<'_> {
        IoStream {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

/// A file-based IO stream for KyTea.
/// It wraps C++ `std::fstream`.
pub struct Fstream {
    inner: *mut c_void,
}

impl Fstream {
    /// Opens a file for reading.
    pub fn open(path: &CStr) -> Result<Self, IoError> {
        unsafe {
            let result = ffi::kytea_fstream_new_path_in(path.as_ptr());
            debug_assert!(!result.file.is_null());

            if let Err(e) = result.err.to_io() {
                ffi::kytea_fstream_delete(result.file);
                Err(e)
            } else {
                debug_assert!(result.err.msg.is_null());
                Ok(Self { inner: result.file })
            }
        }
    }

    /// Creates a file for writing (truncates if exists).
    pub fn create(path: &CStr) -> Result<Self, IoError> {
        unsafe {
            let result = ffi::kytea_fstream_new_path_out(path.as_ptr(), false);
            debug_assert!(!result.file.is_null());

            if let Err(e) = result.err.to_io() {
                ffi::kytea_fstream_delete(result.file);
                Err(e)
            } else {
                debug_assert!(result.err.msg.is_null());
                Ok(Self { inner: result.file })
            }
        }
    }

    /// Opens a file for appending.
    pub fn append(path: &CStr) -> Result<Self, IoError> {
        unsafe {
            let result = ffi::kytea_fstream_new_path_out(path.as_ptr(), true);
            debug_assert!(!result.file.is_null());

            if let Err(e) = result.err.to_io() {
                ffi::kytea_fstream_delete(result.file);
                Err(e)
            } else {
                debug_assert!(result.err.msg.is_null());
                Ok(Self { inner: result.file })
            }
        }
    }

    /// Flushes the internal buffer.
    pub fn flush(&mut self) -> Result<(), IoError> {
        unsafe {
            if self.inner.is_null() {
                return Ok(());
            }

            if !self.inner.is_null()
                && let Err(e) = ffi::kytea_fstream_flush(self.inner).err.to_io()
            {
                Err(e)
            } else {
                Ok(())
            }
        }
    }
}

impl Stream for Fstream {
    fn as_stream(&mut self) -> IoStream<'_> {
        IoStream {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl Drop for Fstream {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                ffi::kytea_fstream_delete(self.inner);
            }
        }
    }
}

/// A span stream for KyTea.
/// It wraps C++ `std::ispanstream`.
pub struct IspanStream<'a> {
    inner: *mut c_void,
    _marker: PhantomData<&'a ()>,
}

impl<'a> IspanStream<'a> {
    /// Creates a new empty [`IspanStream`].
    pub fn new(buf: &'a [u8]) -> Self {
        unsafe {
            let inner = ffi::kytea_ispanstream_new(buf.as_ptr() as *const _, buf.len());
            debug_assert!(!inner.is_null());
            Self {
                inner,
                _marker: PhantomData,
            }
        }
    }
}

impl Drop for IspanStream<'_> {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                ffi::kytea_ispanstream_delete(self.inner);
            }
        }
    }
}

impl Stream for IspanStream<'_> {
    fn as_stream(&mut self) -> IoStream<'_> {
        IoStream {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

/// An execution context for KyTea prediction or training.
pub struct Context<'k, In, Out> {
    input: CorpusIo<In>,
    output: CorpusIo<Out>,
    kytea: &'k mut KyTea,
}

struct CorpusIo<Io> {
    inner: *mut c_void,
    _marker: PhantomData<Io>,
}

impl<Io> Drop for CorpusIo<Io> {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                ffi::kytea_corpus_io_delete(self.inner);
            }
        }
    }
}

impl<In, Out> Context<'_, In, Out> {
    /// Performs prediction for the next sentence/item in the input stream.
    ///
    /// Returns [`ControlFlow::Break(())`](ControlFlow) if the input stream is exhausted.
    pub fn predict(&mut self) -> Result<ControlFlow<()>, IoError> {
        unsafe {
            let result =
                ffi::kytea_model_predict(self.kytea.inner, self.input.inner, self.output.inner);

            if let Err(e) = result.err.to_io() {
                Err(e)
            } else if result.ended {
                Ok(ControlFlow::Break(()))
            } else {
                Ok(ControlFlow::Continue(()))
            }
        }
    }
}
