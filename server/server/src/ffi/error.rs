use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

// Thread-local so an error recorded by one FFI call cannot be read by another thread's
// `bvc_get_last_error`, which would report a fault the caller never triggered.
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

pub(super) struct FfiError;

impl FfiError {
    pub(super) fn set_last_error(msg: &str) {
        LAST_ERROR.with(|e| {
            *e.borrow_mut() = CString::new(msg).ok();
        });
    }

    // The pointer stays valid only until the next FFI call on this thread, which is the
    // contract `bvc_get_last_error` documents to its caller.
    pub(super) fn last_error_ptr() -> *const c_char {
        LAST_ERROR.with(|e| {
            e.borrow()
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null())
        })
    }
}
