use std::ffi::{c_char, c_int, c_uint, c_void};
use std::io::Error as IoError;

#[repr(C)]
pub(crate) struct Str {
    pub(crate) ptr: *const c_char,
    pub(crate) size: usize,
}

#[repr(C)]
pub(crate) struct Err {
    pub(crate) msg: *const c_char,
    pub(crate) code: c_int,
}

#[repr(C)]
pub(crate) struct FileResult {
    pub(crate) file: *mut c_void,
    pub(crate) err: Err,
}

#[repr(C)]
pub(crate) struct PredResult {
    pub(crate) ended: bool,
    pub(crate) err: Err,
}

impl Err {
    pub(crate) fn to_io(&self) -> Result<(), IoError> {
        use std::ffi::CStr;

        if self.msg.is_null() && self.code == 0 {
            Ok(())
        } else if self.code == 255 {
            let ret = unsafe {
                let err = CStr::from_ptr(self.msg);
                IoError::other(String::from_utf8_lossy(err.to_bytes()))
            };
            unsafe {
                kytea_free_err_message(self.msg);
            }
            Err(ret)
        } else {
            unsafe {
                kytea_free_err_message(self.msg);
            }
            Err(IoError::from_raw_os_error(self.code))
        }
    }
}

#[repr(C)]
pub(crate) enum CorpusFormat {
    Raw,
    Full,
    Part,
    Prob,
    Tok,
    _Default,
    Eda,
    Tags,
}

#[repr(u8)]
pub(crate) enum ModelFormat {
    Binary = b'B',
    Text = b'T',
    Unknown = b'U',
}

unsafe extern "C" {
    pub(crate) fn kytea_version() -> *const c_char;

    pub(crate) fn kytea_free_err_message(err: *const c_char);

    pub(crate) fn kytea_model_new() -> *mut c_void;
    pub(crate) fn kytea_model_delete(kytea: *mut c_void);
    pub(crate) fn kytea_model_read_path(
        kytea: *mut c_void,
        model: *const c_char,
        format: ModelFormat,
    ) -> Err;
    pub(crate) fn kytea_model_read_stream(
        kytea: *mut c_void,
        model: *mut c_void,
        format: ModelFormat,
    ) -> Err;
    pub(crate) fn kytea_model_sanity_train(kytea: *mut c_void) -> c_int;
    pub(crate) fn kytea_model_prepare_train(kytea: *mut c_void, output: *mut c_void) -> c_int;
    pub(crate) fn kytea_model_config(kytea: *mut c_void) -> *mut c_void;

    pub(crate) fn kytea_config_set_debug(config: *mut c_void, level: c_uint);
    pub(crate) fn kytea_config_set_training(config: *mut c_void, flag: bool);
    pub(crate) fn kytea_config_set_word_bound(config: *mut c_void, word_bound: *const c_char);
    pub(crate) fn kytea_config_set_tag_bound(config: *mut c_void, tag_bound: *const c_char);
    pub(crate) fn kytea_config_set_elem_bound(config: *mut c_void, elem_bound: *const c_char);
    pub(crate) fn kytea_config_set_unk_bound(config: *mut c_void, unk_bound: *const c_char);
    pub(crate) fn kytea_config_set_no_bound(config: *mut c_void, no_bound: *const c_char);
    pub(crate) fn kytea_config_set_has_bound(config: *mut c_void, has_bound: *const c_char);
    pub(crate) fn kytea_config_set_skip_bound(config: *mut c_void, skip_bound: *const c_char);
    pub(crate) fn kytea_config_set_escape(config: *mut c_void, escape: *const c_char);

    pub(crate) fn kytea_config_set_input_format(config: *mut c_void, fmt: CorpusFormat);
    pub(crate) fn kytea_config_set_do_ws(config: *mut c_void, do_ws: bool);

    pub(crate) fn kytea_stringstream_new() -> *mut c_void;
    pub(crate) fn kytea_stringstream_delete(buf: *mut c_void);
    pub(crate) fn kytea_stringstream_as_slice(buf: *mut c_void) -> Str;
    pub(crate) fn kytea_stringstream_write(buf: *mut c_void, input: Str);

    pub(crate) fn kytea_fstream_new_path_in(path: *const c_char) -> FileResult;
    pub(crate) fn kytea_fstream_new_path_out(path: *const c_char, append: bool) -> FileResult;
    pub(crate) fn kytea_fstream_delete(file: *mut c_void);
    pub(crate) fn kytea_fstream_flush(file: *mut c_void) -> FileResult;

    pub(crate) fn kytea_ispanstream_new(ptr: *const c_char, len: usize) -> *mut c_void;
    pub(crate) fn kytea_ispanstream_delete(stream: *mut c_void);

    pub(crate) fn kytea_model_corpus(
        kytea: *mut c_void,
        corpus: *mut c_void,
        is_output: bool,
    ) -> *mut c_void;
    pub(crate) fn kytea_corpus_io_delete(corpus: *mut c_void);

    pub(crate) fn kytea_model_predict(
        kytea: *mut c_void,
        input: *mut c_void,
        output: *mut c_void,
    ) -> PredResult;
}

pub mod defaults {
    use std::ffi::CStr;

    pub const WORD_BOUND: &CStr = c" ";
    pub const TAG_BOUND: &CStr = c"/";
    pub const ELEM_BOUND: &CStr = c"&";
    pub const UNK_BOUND: &CStr = c" ";
    pub const NO_BOUND: &CStr = c"-";
    pub const HAS_BOUND: &CStr = c"|";
    pub const SKIP_BOUND: &CStr = c"?";
    pub const ESCAPE: &CStr = c"\\";
}
