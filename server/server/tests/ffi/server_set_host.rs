use bvc_server_lib::ffi::{
    bvc_server_create, bvc_server_destroy, bvc_server_set_host,
};
use std::ptr;

const MINIMAL_CONFIG: &std::ffi::CStr = c"{\"server\":{\"port\":8444,\"quic_port\":8443}}";

#[test]
fn set_host_on_a_created_handle_returns_zero() {
    let handle = unsafe { bvc_server_create(MINIMAL_CONFIG.as_ptr()) };
    assert!(!handle.is_null());

    let result = unsafe { bvc_server_set_host(handle, c"paper".as_ptr()) };
    assert_eq!(result, 0);

    unsafe { bvc_server_destroy(handle) };
}

#[test]
fn set_host_with_a_null_handle_or_kind_returns_minus_one() {
    assert_eq!(
        unsafe { bvc_server_set_host(ptr::null_mut(), c"paper".as_ptr()) },
        -1
    );

    let handle = unsafe { bvc_server_create(MINIMAL_CONFIG.as_ptr()) };
    assert!(!handle.is_null());
    assert_eq!(unsafe { bvc_server_set_host(handle, ptr::null()) }, -1);
    unsafe { bvc_server_destroy(handle) };
}
