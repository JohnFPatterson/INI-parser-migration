//! ABI smoke tests for inih-ffi (link via rlib exports).
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use inih_ffi::{ini_parse, ini_parse_string, ini_parse_string_length};

unsafe extern "C" fn ok_handler(
    _user: *mut c_void,
    _section: *const c_char,
    _name: *const c_char,
    _value: *const c_char,
) -> c_int {
    1
}

#[test]
fn ffi_parse_string_basic() {
    let s = CString::new("[a]\nx=y\n").unwrap();
    // SAFETY: valid C string and handler.
    let e = unsafe { ini_parse_string(s.as_ptr(), Some(ok_handler), ptr::null_mut()) };
    assert_eq!(e, 0);
}

#[test]
fn ffi_parse_missing_file() {
    let s = CString::new("__inih_no_such_file__.ini").unwrap();
    // SAFETY: valid C string and handler.
    let e = unsafe { ini_parse(s.as_ptr(), Some(ok_handler), ptr::null_mut()) };
    assert_eq!(e, -1);
}

#[test]
fn ffi_parse_string_length() {
    let s = b"[a]\nx=y\n";
    // SAFETY: buffer length matches `s.len()`.
    let e = unsafe {
        ini_parse_string_length(
            s.as_ptr() as *const c_char,
            s.len() as u64,
            Some(ok_handler),
            ptr::null_mut(),
        )
    };
    assert_eq!(e, 0);
}
