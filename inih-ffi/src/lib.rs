//! Thin C ABI shim for inih-core.
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use inih_core::{
    ini_parse_file_path, ini_parse_string_length as core_parse_string_length, IniConfig, IniHandler,
};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;
use std::slice;

#[cfg(feature = "handler-lineno")]
pub type IniHandlerFn = Option<
    unsafe extern "C" fn(
        user: *mut c_void,
        section: *const c_char,
        name: *const c_char,
        value: *const c_char,
        lineno: c_int,
    ) -> c_int,
>;

#[cfg(not(feature = "handler-lineno"))]
pub type IniHandlerFn = Option<
    unsafe extern "C" fn(
        user: *mut c_void,
        section: *const c_char,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int,
>;

pub type IniReaderFn =
    Option<unsafe extern "C" fn(str: *mut c_char, num: c_int, stream: *mut c_void) -> *mut c_char>;

fn feature_config() -> IniConfig {
    IniConfig {
        allow_multiline: cfg!(feature = "multi-line"),
        allow_bom: cfg!(feature = "utf8-bom"),
        allow_inline_comments: cfg!(feature = "inline-comments"),
        use_stack: cfg!(feature = "use-stack") && !cfg!(feature = "use-heap"),
        allow_realloc: cfg!(feature = "allow-realloc"),
        stop_on_first_error: cfg!(feature = "stop-on-first-error"),
        handler_lineno: cfg!(feature = "handler-lineno"),
        call_handler_on_new_section: cfg!(feature = "call-handler-on-new-section"),
        allow_no_value: cfg!(feature = "allow-no-value"),
        ..Default::default()
    }
}

fn cstring_lossy(s: &str) -> CString {
    CString::new(
        s.as_bytes()
            .iter()
            .copied()
            .filter(|&b| b != 0)
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default()
}

struct CHandler {
    f: IniHandlerFn,
    user: *mut c_void,
}

impl IniHandler for CHandler {
    fn handle(
        &mut self,
        section: &str,
        name: Option<&str>,
        value: Option<&str>,
        lineno: i32,
    ) -> bool {
        let Some(f) = self.f else {
            return false;
        };
        let section_c = cstring_lossy(section);
        let name_c = name.map(cstring_lossy);
        let value_c = value.map(cstring_lossy);
        let name_ptr = name_c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null());
        let value_ptr = value_c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null());
        // SAFETY: Invokes the caller-provided C handler with pointers that remain valid for the call.
        let rc = unsafe {
            #[cfg(feature = "handler-lineno")]
            {
                f(
                    self.user,
                    section_c.as_ptr(),
                    name_ptr,
                    value_ptr,
                    lineno as c_int,
                )
            }
            #[cfg(not(feature = "handler-lineno"))]
            {
                let _ = lineno;
                f(self.user, section_c.as_ptr(), name_ptr, value_ptr)
            }
        };
        rc != 0
    }
}

fn read_all_via_reader(reader: IniReaderFn, stream: *mut c_void, chunk: usize) -> Vec<u8> {
    let Some(reader) = reader else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut buf = vec![0i8; chunk.max(4)];
    loop {
        // SAFETY: `buf` is valid for `chunk` writes; `stream` is owned by the caller.
        let p = unsafe { reader(buf.as_mut_ptr(), buf.len() as c_int, stream) };
        if p.is_null() {
            break;
        }
        // SAFETY: reader NUL-terminated the buffer like fgets.
        let s = unsafe { CStr::from_ptr(buf.as_ptr()) };
        out.extend_from_slice(s.to_bytes());
    }
    out
}

/// Opaque FILE* stand-in.
#[repr(C)]
pub struct LibcFile {
    _private: [u8; 0],
}

extern "C" {
    fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut LibcFile) -> usize;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut LibcFile;
    fn fclose(file: *mut LibcFile) -> c_int;
}

unsafe fn read_c_file(file: *mut LibcFile) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        // SAFETY: `file` is a valid FILE*; `buf` is writable.
        let n = unsafe { fread(buf.as_mut_ptr().cast(), 1, buf.len(), file) };
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    out
}

/// # Safety
/// `reader`/`stream` must be valid for the parse; `handler` may be invoked with transient pointers.
#[no_mangle]
pub unsafe extern "C" fn ini_parse_stream(
    reader: IniReaderFn,
    stream: *mut c_void,
    handler: IniHandlerFn,
    user: *mut c_void,
) -> c_int {
    let cfg = feature_config();
    // SAFETY: reader/stream contract matches C `ini_parse_stream`.
    let data = read_all_via_reader(reader, stream, cfg.max_line.max(64));
    let mut ch = CHandler { f: handler, user };
    core_parse_string_length(&cfg, &data, &mut ch) as c_int
}

/// # Safety
/// `file` must be a valid open `FILE*`; caller retains ownership.
#[no_mangle]
pub unsafe extern "C" fn ini_parse_file(
    file: *mut LibcFile,
    handler: IniHandlerFn,
    user: *mut c_void,
) -> c_int {
    if file.is_null() {
        return -1;
    }
    // SAFETY: `file` is a valid FILE*.
    let data = unsafe { read_c_file(file) };
    let cfg = feature_config();
    let mut ch = CHandler { f: handler, user };
    core_parse_string_length(&cfg, &data, &mut ch) as c_int
}

/// # Safety
/// `filename` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn ini_parse(
    filename: *const c_char,
    handler: IniHandlerFn,
    user: *mut c_void,
) -> c_int {
    if filename.is_null() {
        return -1;
    }
    // SAFETY: `filename` is a valid C string.
    let path = unsafe { CStr::from_ptr(filename) };
    let Ok(path) = path.to_str() else {
        return -1;
    };
    let cfg = feature_config();
    let mut ch = CHandler { f: handler, user };
    ini_parse_file_path(&cfg, Path::new(path), &mut ch) as c_int
}

/// # Safety
/// `string` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ini_parse_string(
    string: *const c_char,
    handler: IniHandlerFn,
    user: *mut c_void,
) -> c_int {
    if string.is_null() {
        return 0;
    }
    // SAFETY: NUL-terminated C string.
    let s = unsafe { CStr::from_ptr(string) };
    let cfg = feature_config();
    let mut ch = CHandler { f: handler, user };
    core_parse_string_length(&cfg, s.to_bytes(), &mut ch) as c_int
}

/// # Safety
/// `string` must point to at least `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn ini_parse_string_length(
    string: *const c_char,
    length: usize,
    handler: IniHandlerFn,
    user: *mut c_void,
) -> c_int {
    if string.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees `length` bytes at `string`.
    let bytes = unsafe { slice::from_raw_parts(string.cast::<u8>(), length) };
    let cfg = feature_config();
    let mut ch = CHandler { f: handler, user };
    core_parse_string_length(&cfg, bytes, &mut ch) as c_int
}

#[allow(dead_code)]
fn _link_stdio() {
    let _ = (fopen, fclose);
}
